use super::ChainMessage;
use crate::models::ChainBlock;
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    rpc::types::{Filter, Log},
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolEvent,
    transports::http::reqwest::Url,
};
use anyhow::Result;
use redis::{AsyncCommands, Client as RedisClient};
use sqlx::PgPool;
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{Duration, sleep},
};

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    EvmToken,
    "ERC20.json"
);

#[derive(Debug)]
pub struct TransferEvent {
    pub token: Address,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub transaction_hash: B256,
    // pub block_number: u64,
    // pub log_index: u64,
}

// Scanner state to track progress
#[derive(Debug)]
pub struct Scanner {
    db: PgPool,
    chain_id: usize,
    name: String,
    latency: u64,
    rpc: Url,
    tokens: Vec<Address>,
    event: B256,
    last_scanned_block: u64,
    sender: UnboundedSender<ChainMessage>,
    redis: RedisClient,
}

impl Scanner {
    pub async fn new(
        db: PgPool,
        chain_id: usize,
        name: String,
        latency: u64,
        rpc: Url,
        tokens: Vec<Address>,
        sender: UnboundedSender<ChainMessage>,
        redis: RedisClient,
    ) -> Result<Self> {
        let event = EvmToken::Transfer::SIGNATURE_HASH;

        // fetch last scanned block from chain
        let last_scanned_block = ChainBlock::get_block(&name, &db).await;

        let mut scan = Self {
            db,
            chain_id,
            name,
            latency,
            rpc,
            tokens,
            event,
            last_scanned_block,
            sender,
            redis,
        };

        if scan.last_scanned_block == 0 {
            scan.last_scanned_block = scan.get_latest_block().await?;
        }

        Ok(scan)
    }

    // Get the latest block number from the chain
    async fn get_latest_block(&self) -> Result<u64> {
        let provider = ProviderBuilder::new().connect_http(self.rpc.clone());
        let block_number = provider.get_block_number().await?;
        Ok(block_number)
    }

    // Scan for transfer events in a block range
    async fn scan_range(&self, from_block: u64, to_block: u64) -> Result<()> {
        let provider = ProviderBuilder::new().connect_http(self.rpc.clone());

        // Create filter for Transfer events from our monitored tokens
        let filter = Filter::new()
            .address(self.tokens.clone())
            .event_signature(self.event)
            .from_block(from_block)
            .to_block(to_block);

        let logs = provider.get_logs(&filter).await?;

        for log in logs {
            if let Ok(event) = self.parse_transfer_event(log).await {
                self.handle_transfer_event(event);
            }
        }

        Ok(())
    }

    // Parse a log into a TransferEvent
    async fn parse_transfer_event(&self, log: Log) -> Result<TransferEvent> {
        // ERC20 Transfer event signature: Transfer(address,address,uint256)
        let event = EvmToken::Transfer::decode_log(&log.inner)?;

        // Check if the 'to' address is a valid customer address
        if !self.is_customer_address(&event.to).await {
            return Err(anyhow::anyhow!("No need"));
        }

        Ok(TransferEvent {
            token: log.address(),
            from: event.from,
            to: event.to,
            amount: event.value,
            transaction_hash: log.transaction_hash.unwrap_or(B256::ZERO),
            // block_number: log.block_number.unwrap_or(0),
            // log_index: log.log_index.unwrap_or(0),
        })
    }

    // Check if an address is a valid customer address in Redis
    async fn is_customer_address(&self, address: &Address) -> bool {
        match self.redis.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                let key = format!("customer_addr:{}", address);
                match conn.exists(&key).await {
                    Ok(exists) => exists,
                    Err(e) => {
                        error!("Redis error checking address {}: {}", address, e);
                        false
                    }
                }
            }
            Err(e) => {
                error!("Failed to get Redis connection: {}", e);
                false
            }
        }
    }

    // Process transfer events and send to the message queue
    fn handle_transfer_event(&self, event: TransferEvent) {
        info!(
            "Chain {}: customer transfer - Token: {}, From: {}, To: {}, Amount: {}",
            self.chain_id, event.token, event.from, event.to, event.amount
        );

        // Send deposit message for processing
        if let Err(e) = self.sender.send(ChainMessage::Deposit(
            self.chain_id,
            event.token,
            event.to,
            event.amount,
            event.transaction_hash,
        )) {
            error!("Failed to send deposit message: {}", e);
        }
    }

    // Single scan iteration
    async fn scan_iteration(&mut self, max_blocks_per_scan: u64) -> Result<u64> {
        // IMPORTANT: for better finalized, we slower some-block, works for almost blockchain
        let latest_block = self.get_latest_block().await? - self.latency;

        if latest_block <= self.last_scanned_block {
            return Ok(0);
        }

        let from_block = self.last_scanned_block + 1;
        let to_block = std::cmp::min(from_block + max_blocks_per_scan, latest_block);

        self.scan_range(from_block, to_block).await?;
        let _ = ChainBlock::insert(&self.name, to_block, &self.db).await;

        let scanned_blocks = to_block - from_block + 1;
        self.last_scanned_block = to_block;

        Ok(scanned_blocks)
    }

    // start scanning loop
    pub fn run(mut self) {
        tokio::spawn(async move {
            let max_blocks_per_scan = 100u64; // Limit blocks per scan to avoid RPC timeouts

            loop {
                let scan_interval = match self.scan_iteration(max_blocks_per_scan).await {
                    Ok(scanned_blocks) => {
                        if scanned_blocks > 0 {
                            debug!(
                                "Chain {}: Scanned {} blocks, current block: {}",
                                self.chain_id, scanned_blocks, self.last_scanned_block
                            );

                            // If we're catching up, scan faster
                            if scanned_blocks >= max_blocks_per_scan {
                                Duration::from_secs(1)
                            } else {
                                // Normal scanning interval
                                Duration::from_secs(10)
                            }
                        } else {
                            // No new blocks, increase interval slightly
                            Duration::from_secs(15)
                        }
                    }
                    Err(e) => {
                        error!("Chain {}: Scan error: {}", self.chain_id, e);
                        // On error, wait longer before retrying
                        Duration::from_secs(30)
                    }
                };

                sleep(scan_interval).await;
            }
        });
    }
}

// transfer token from deposit to admin, return real merchant amount
pub async fn transfer(
    customer: Address,
    merchant: Address,
    token: Address,
    wallet: PrivateKeySigner,
    main: PrivateKeySigner,
    url: Url,
    commission_rate: i32,
) -> Result<(U256, B256)> {
    let maccount = main.address();
    let provider = ProviderBuilder::new()
        .wallet(main)
        .connect_http(url.clone());
    let contract = EvmToken::new(token, provider.clone());

    // 1. check token balance
    let balance: U256 = contract.balanceOf(customer).call().await?;

    if balance == U256::default() {
        return Err(anyhow::anyhow!("No balance"));
    }

    // 3. check approve or not
    let approved: U256 = contract.allowance(customer, maccount).call().await?;
    let need_approve = approved < balance;

    // 2. collect gas used, and do a discount in the amount
    let _transfer_gas = contract
        .transferFrom(customer, maccount, balance)
        .estimate_gas()
        .await?;
    let approve_gas = if need_approve {
        contract
            .approve(maccount, U256::from(100000000_000000i64))
            .estimate_gas()
            .await?
    } else {
        0
    };
    let commission = balance * U256::from(commission_rate) / U256::from(100);
    let fee = commission;
    // + U256::from(transfer_gas * 2 + approve_gas); // TODO fetch current price

    let real = balance - fee;

    if need_approve {
        // 4. if not approve, transfer approve gas to it
        let ttx = TransactionRequest::default()
            .with_to(customer)
            .with_value(U256::from(approve_gas));
        let pending = provider.send_transaction(ttx).await?;
        let _receipt = pending.get_receipt().await?;

        // 5. approve tokens to max
        let customer_provider = ProviderBuilder::new().wallet(wallet).connect_http(url);
        let customer_contract = EvmToken::new(token, customer_provider);

        let pending = customer_contract
            .approve(maccount, U256::from(100000000_000000i64))
            .send()
            .await?;
        let _receipt = pending.get_receipt().await?;
    }

    // 6. transfer remain token to merchant
    let pending = contract
        .transferFrom(customer, merchant, real)
        .send()
        .await?;
    let receipt = pending.get_receipt().await?;

    if fee > U256::from(0) {
        let pending2 = contract
            .transferFrom(customer, maccount, fee)
            .send()
            .await?;
        let _ = pending2.get_receipt().await?;
    }

    Ok((real, receipt.transaction_hash))
}

pub async fn get_token_decimal(token: Address, provider: impl Provider) -> Result<u8> {
    let contract = EvmToken::new(token, provider);
    Ok(contract.decimals().call().await?)
}

pub fn u256_to_i32(amount: U256, decimal: &u8) -> i32 {
    let res = if *decimal > 2 {
        amount / U256::from(*decimal - 2)
    } else {
        amount / U256::from(*decimal)
    };

    res.try_into().unwrap_or(0)
}
