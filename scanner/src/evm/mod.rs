mod pay;
mod sub;

pub use pay::transfer;
pub use sub::claim;

use crate::{Chain, ChainDeposit, ScannerMessage};
use alloy::{
    primitives::{Address, B256, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::{Filter, Log},
    sol,
    sol_types::SolEvent,
    transports::http::reqwest::Url,
};
use anyhow::Result;
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

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    EvmSubscription,
    "Subscription.json"
);

// Scanner state to track progress
#[derive(Debug)]
pub struct Scanner {
    index: usize,
    latency: u64,
    rpc: Url,
    contracts: Vec<Address>,
    events: Vec<B256>,
    last_scanned_block: u64,
    sender: UnboundedSender<ScannerMessage>,
}

impl Scanner {
    pub async fn new(
        index: usize,
        chain: &Chain,
        sender: UnboundedSender<ScannerMessage>,
    ) -> Result<Self> {
        let mut events = vec![EvmToken::Transfer::SIGNATURE_HASH];
        let mut contracts = chain.tokens.keys().copied().collect();

        if let Some(s) = chain.subscription {
            events.extend([
                EvmSubscription::PlanStarted,
                EvmSubscription::PlanCanceled,
                EvmSubscription::SubscriptionStarted,
                EvmSubscription::SubscriptionCanceled,
                EvmSubscription::SubscriptionClaimed,
            ]);
            contracts.push(s);
        }

        let mut scan = Self {
            index,
            latency: chain.latency as u64,
            rpc: chain.rpc.clone(),
            contracts,
            events,
            last_scanned_block: chain.last_scanned_block as u64,
            sender,
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
            .address(self.contracts.clone())
            .event_signature(self.events.clone())
            .from_block(from_block)
            .to_block(to_block);

        let logs = provider.get_logs(&filter).await?;
        for log in logs {
            if let Err(err) = self.handle_event(log) {
                tracing::error!("Parse event error: {:?}", err);
            }
        }

        Ok(())
    }

    // Parse a log into a TransferEvent
    fn handle_event(&self, log: Log) -> Result<()> {
        // ERC20 Transfer event signature: Transfer(address,address,uint256)

        if let Ok(event) = EvmToken::Transfer::decode_log(&log.inner) {
            tracing::debug!(
                "Fetch transfer: {}-{}:{}",
                event.from,
                event.to,
                event.value
            );

            // Send deposit message for processing
            let _ = self.sender.send(ScannerMessage::Deposit(
                self.index,
                ChainDeposit::Evm(
                    log.address(), // token address
                    event.to,
                    event.value,
                    log.transaction_hash.unwrap_or(B256::ZERO), // tx hash
                ),
            ));

            // block_number: log.block_number.unwrap_or(0),
            // log_index: log.log_index.unwrap_or(0),
            return Ok(());
        }

        if let Ok(event) = EvmSubscription::SubscriptionClaimed::decode_log(&log.inner) {
            tracing::debug!("Fetch subscription claimed: {}-{}:{}", event.id);

            // Send deposit message for processing
            let _ = self.sender.send(ScannerMessage::Subscription(
                self.index,
                ChainSubscription::Claimed(
                    event.id,
                    log.transaction_hash.unwrap_or(B256::ZERO), // tx hash
                ),
            ));

            return Ok(());
        }

        if let Ok(event) = EvmSubscription::PlanStarted::decode_log(&log.inner) {
            tracing::debug!("Fetch plan started: {}:{}", event.id, event.merchant);

            // Send deposit message for processing
            let _ = self.sender.send(ScannerMessage::Plan(
                self.index,
                ChainPlan::PlanStarted(
                    event.id,
                    event.merchant,
                    event.amount,
                    event.period,
                    log.transaction_hash.unwrap_or(B256::ZERO), // tx hash
                ),
            ));

            return Ok(());
        }

        if let Ok(event) = EvmSubscription::PlanCanceled::decode_log(&log.inner) {
            tracing::debug!("Fetch plan canceled: {}", event.id);

            // Send deposit message for processing
            let _ = self.sender.send(ScannerMessage::Plan(
                self.index,
                ChainPlan::PlanCanceled(
                    event.id,
                    log.transaction_hash.unwrap_or(B256::ZERO), // tx hash
                ),
            ));

            return Ok(());
        }

        if let Ok(event) = EvmSubscription::SubscriptionStarted::decode_log(&log.inner) {
            tracing::debug!("Fetch subscription started: {}", event.id);

            // Send deposit message for processing
            let _ = self.sender.send(ScannerMessage::Plan(
                self.index,
                ChainPlan::SubscriptionStarted(
                    event.id,
                    event.plan,
                    event.customer,
                    event.payer,
                    event.token,
                    event.nextTime,
                    log.transaction_hash.unwrap_or(B256::ZERO), // tx hash
                ),
            ));

            return Ok(());
        }

        if let Ok(event) = EvmSubscription::SubscriptionCanceled::decode_log(&log.inner) {
            tracing::debug!("Fetch subscription canceled: {}", event.id);

            // Send deposit message for processing
            let _ = self.sender.send(ScannerMessage::Plan(
                self.index,
                ChainPlan::SubscriptionCanceled(
                    event.id,
                    log.transaction_hash.unwrap_or(B256::ZERO), // tx hash
                ),
            ));

            return Ok(());
        }

        Ok(())
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
        let _ = self
            .sender
            .send(ScannerMessage::Scanned(self.index, to_block as i64));

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
                            tracing::info!(
                                "Chain {}: Scanned {} blocks, current block: {}",
                                self.index,
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
                        tracing::error!("Chain {}: Scan error: {}", self.index, e);
                        // On error, wait longer before retrying
                        Duration::from_secs(30)
                    }
                };

                sleep(scan_interval).await;
            }
        });
    }
}

pub async fn get_token_decimal(token: Address, provider: impl Provider) -> Result<u8> {
    let contract = EvmToken::new(token, provider);
    Ok(contract.decimals().call().await?)
}

pub fn u256_to_i32(amount: U256, decimal: &u8) -> i32 {
    let res = if *decimal > 2 {
        amount / U256::from(10).pow(U256::from(*decimal - 2))
    } else {
        amount * U256::from(10).pow(U256::from(2 - *decimal))
    };

    res.try_into().unwrap_or(0)
}

pub fn i32_to_u256(amount: i32, decimal: &u8) -> U256 {
    if *decimal > 2 {
        U256::from(amount) * U256::from(10).pow(U256::from(*decimal - 2))
    } else {
        U256::from(amount) / U256::from(10).pow(U256::from(2 - *decimal))
    }
}
