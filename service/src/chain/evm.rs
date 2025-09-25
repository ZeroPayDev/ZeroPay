use super::ChainMessage;
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    rpc::types::{Filter, Log},
    signers::local::PrivateKeySigner,
    sol,
    transports::http::reqwest::Url,
};
use anyhow::Result;
use redis::{AsyncCommands, Client as RedisClient};
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

// ERC20 Transfer event signature: Transfer(address,address,uint256)
const TRANSFER_EVENT_SIGNATURE: B256 = B256::new([
    0xdd, 0xf2, 0x52, 0xad, 0x1b, 0xe2, 0xc8, 0x9b, 0x69, 0xc2, 0xb0, 0x68, 0xfc, 0x37, 0x8d, 0xaa,
    0x95, 0x2b, 0xa7, 0xf1, 0x63, 0xc4, 0xa1, 0x16, 0x28, 0xf5, 0x5a, 0x4d, 0xf5, 0x23, 0xb3, 0xef,
]);

#[derive(Debug)]
pub struct TransferEvent {
    pub token: Address,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub block_number: u64,
    pub transaction_hash: B256,
    pub log_index: u64,
}

// Scanner state to track progress
#[derive(Debug)]
struct ScannerState {
    chain_id: usize,
    rpc_url: Url,
    tokens: Vec<Address>,
    last_scanned_block: u64,
    sender: UnboundedSender<ChainMessage>,
    redis: RedisClient,
}

impl ScannerState {
    pub fn new(
        chain_id: usize,
        rpc_url: Url,
        tokens: Vec<Address>,
        last_scanned_block: u64,
        sender: UnboundedSender<ChainMessage>,
        redis: RedisClient,
    ) -> Self {
        Self {
            chain_id,
            rpc_url,
            tokens,
            last_scanned_block,
            sender,
            redis,
        }
    }

    // Get the latest block number from the chain
    async fn get_latest_block(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let provider = ProviderBuilder::new().connect_http(self.rpc_url.clone());
        let block_number = provider.get_block_number().await?;
        Ok(block_number)
    }

    // Scan for transfer events in a block range
    async fn scan_range(
        &self,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<TransferEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let provider = ProviderBuilder::new().connect_http(self.rpc_url.clone());
        let mut events = Vec::new();

        // Create filter for Transfer events from our monitored tokens
        let filter = Filter::new()
            .address(self.tokens.clone())
            .event_signature(TRANSFER_EVENT_SIGNATURE)
            .from_block(from_block)
            .to_block(to_block);

        let logs = provider.get_logs(&filter).await?;

        for log in logs {
            if let Some(event) = self.parse_transfer_event(log) {
                events.push(event);
            }
        }

        // Sort by block number and log index for consistent processing
        events.sort_by(|a, b| {
            a.block_number
                .cmp(&b.block_number)
                .then_with(|| a.log_index.cmp(&b.log_index))
        });

        Ok(events)
    }

    // Parse a log into a TransferEvent
    fn parse_transfer_event(&self, log: Log) -> Option<TransferEvent> {
        // Validate log structure for Transfer event
        let topics = log.topics();
        if topics.len() != 3 {
            return None;
        }

        // Extract addresses from topics
        let from_bytes = topics[1].as_slice();
        let to_bytes = topics[2].as_slice();

        if from_bytes.len() != 32 || to_bytes.len() != 32 {
            return None;
        }

        // Convert to addresses (take last 20 bytes)
        let from = Address::from_slice(&from_bytes[12..32]);
        let to = Address::from_slice(&to_bytes[12..32]);

        // Parse amount from data
        let data = &log.data().data;
        if data.len() != 32 {
            return None;
        }

        let amount = U256::from_be_slice(data);

        Some(TransferEvent {
            token: log.address(),
            from,
            to,
            amount,
            block_number: log.block_number.unwrap_or(0),
            transaction_hash: log.transaction_hash.unwrap_or(B256::ZERO),
            log_index: log.log_index.unwrap_or(0),
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
                        tracing::error!("Redis error checking address {}: {}", address, e);
                        false
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to get Redis connection: {}", e);
                false
            }
        }
    }

    // Process transfer events and send to the message queue
    async fn process_events(&self, events: Vec<TransferEvent>) {
        for event in events {
            // Check if the 'to' address is a valid customer address
            let is_customer = self.is_customer_address(&event.to).await;

            if is_customer {
                tracing::info!(
                    "Chain {}: Valid customer transfer detected - Token: {}, From: {}, To: {}, Amount: {}, Block: {}",
                    self.chain_id,
                    event.token,
                    event.from,
                    event.to,
                    event.amount,
                    event.block_number
                );

                // Send deposit message for processing
                if let Err(e) = self.sender.send(ChainMessage::Deposit(
                    self.chain_id,
                    event.token,
                    event.to,
                    event.amount,
                    event.block_number,
                    event.transaction_hash,
                )) {
                    tracing::error!("Failed to send deposit message: {}", e);
                }
            } else {
                tracing::debug!(
                    "Chain {}: Transfer to non-customer address ignored - To: {}, Amount: {}",
                    self.chain_id,
                    event.to,
                    event.amount
                );
            }
        }
    }

    // Main scanning loop
    async fn run_scan_loop(&mut self) {
        let max_blocks_per_scan = 1000u64; // Limit blocks per scan to avoid RPC timeouts

        loop {
            let scan_interval = match self.scan_iteration(max_blocks_per_scan).await {
                Ok(scanned_blocks) => {
                    if scanned_blocks > 0 {
                        tracing::debug!(
                            "Chain {}: Scanned {} blocks, current block: {}",
                            self.chain_id,
                            scanned_blocks,
                            self.last_scanned_block
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
                    tracing::error!("Chain {}: Scan error: {}", self.chain_id, e);
                    // On error, wait longer before retrying
                    Duration::from_secs(30)
                }
            };

            sleep(scan_interval).await;
        }
    }

    // Single scan iteration
    async fn scan_iteration(
        &mut self,
        max_blocks_per_scan: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let latest_block = self.get_latest_block().await?;

        if latest_block <= self.last_scanned_block {
            return Ok(0);
        }

        let from_block = self.last_scanned_block + 1;
        let to_block = std::cmp::min(from_block + max_blocks_per_scan - 1, latest_block);

        let events = self.scan_range(from_block, to_block).await?;
        self.process_events(events).await;

        let scanned_blocks = to_block - from_block + 1;
        self.last_scanned_block = to_block;

        Ok(scanned_blocks)
    }
}

// scan the tokens transfer
pub fn scan(
    id: usize,
    rpc: Url,
    tokens: Vec<Address>,
    sender: UnboundedSender<ChainMessage>,
    redis: RedisClient,
) {
    tokio::spawn(async move {
        // Get the latest block as starting point
        let provider = ProviderBuilder::new().connect_http(rpc.clone());
        let latest_block = match provider.get_block_number().await {
            Ok(block) => block,
            Err(e) => {
                tracing::error!("Chain {}: Failed to get latest block: {}", id, e);
                return;
            }
        };

        tracing::info!(
            "Chain {}: Starting EVM scanner from block {} for {} tokens",
            id,
            latest_block,
            tokens.len()
        );

        // Start scanner from latest block (for new deployments)
        // In production, you might want to load this from database
        let mut scanner = ScannerState::new(id, rpc, tokens, latest_block, sender, redis);

        scanner.run_scan_loop().await;
    });
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
