mod did;
mod event;
mod evm;

pub use did::generate_eth;
pub use event::ScannerEvent;

use alloy::{
    primitives::{Address, B256, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// Chain configure
#[derive(Debug, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub chains: Vec<ChainConfig>,
}

/// Chain configure
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_type: String,
    pub chain_name: String,
    pub latency: i32,
    pub estimation: i32,
    pub commission: i32,
    pub commission_min: i32,
    pub commission_max: i32,
    pub rpc: String,
    pub admin: Option<String>,
    pub tokens: Vec<String>,
}

/// Main storage interface for Scanner used
pub trait ScannerStorage: Send + Sync + 'static {
    fn get_scanned_block(&self, name: &str) -> impl Future<Output = Result<i64>> + Send;
    fn set_scanned_block(&self, name: &str, block: i64) -> impl Future<Output = Result<()>> + Send;
    fn contains_address(
        &self,
        address: &str,
    ) -> impl Future<Output = Result<(i32, i32, String)>> + Send;
    fn no_transaction(&self, tx: &str) -> impl Future<Output = Result<()>> + Send;
    fn deposited(
        &self,
        identity: String,
        mid: i32,
        cid: i32,
        amount: i32,
        tx: String,
    ) -> impl Future<Output = Result<i32>> + Send;
    fn settled(
        &self,
        identity: String,
        did: i32,
        amount: i32,
        tx: String,
    ) -> impl Future<Output = Result<()>> + Send;
}

enum ChainType {
    Evm,
}

impl ChainType {
    fn from_str(s: &str) -> ChainType {
        match s.to_lowercase().as_str() {
            "evm" => ChainType::Evm,
            _ => ChainType::Evm,
        }
    }
}

struct Chain {
    chain_type: ChainType,
    chain_name: String,
    latency: i64,
    commission: i32,
    commission_min: i32,
    commission_max: i32,
    rpc: Url,
    wallet: PrivateKeySigner,
    tokens: HashMap<Address, String>,
    decimals: HashMap<Address, u8>,
    last_scanned_block: i64,
}

pub enum ChainDeposit {
    // token_address, to_address, amount, tx_hash
    Evm(Address, Address, U256, B256),
}

/// Scanner service message
pub enum ScannerMessage {
    /// new deposit, chain_id, deposit
    Deposit(usize, ChainDeposit),
    /// scanned block number
    Scanned(usize, i64),
}

pub struct ScannerService<S: ScannerStorage> {
    storage: S,
    mnemonics: String,
    chains: Vec<Chain>,
}

impl<S: ScannerStorage> ScannerService<S> {
    pub async fn new(storage: S, mnemonics: String, config: ScannerConfig) -> Result<Self> {
        // parse the chain configure
        let (default_sk, _addr) = generate_eth(0, 0, &mnemonics)?;
        let default_admin: PrivateKeySigner = default_sk.parse()?;
        let mut chains = vec![];
        for config in config.chains {
            let chain_type = ChainType::from_str(&config.chain_type);
            let wallet: PrivateKeySigner = if let Some(admin) = config.admin {
                admin.parse()?
            } else {
                default_admin.clone()
            };
            let rpc: Url = config.rpc.parse()?;
            let provider = ProviderBuilder::new().connect_http(rpc.clone());

            // fetch token decimal and also test the rpc is work
            let mut tokens = HashMap::new();
            let mut decimals = HashMap::new();
            for t in config.tokens.iter() {
                let mut values = t.split(":");
                let name: String = values.next().unwrap_or_default().to_owned();
                let token: Address = values.next().unwrap_or_default().parse()?;
                let decimal = evm::get_token_decimal(token, provider.clone()).await?;
                let identity = format!("{}:{}", config.chain_name, name);
                tokens.insert(token, identity);
                decimals.insert(token, decimal);
            }

            let last_scanned_block = storage.get_scanned_block(&config.chain_name).await?;

            chains.push(Chain {
                chain_type,
                chain_name: config.chain_name,
                latency: config.latency as i64,
                commission: config.commission,
                commission_min: config.commission_min,
                commission_max: config.commission_max,
                rpc,
                wallet,
                tokens,
                decimals,
                last_scanned_block,
            });
        }

        Ok(Self {
            storage,
            mnemonics,
            chains,
        })
    }

    pub async fn run(self) -> Result<UnboundedSender<ScannerMessage>> {
        let (sender, receiver) = unbounded_channel::<ScannerMessage>();

        // start chain scanners
        for (i, chain) in self.chains.iter().enumerate() {
            match chain.chain_type {
                ChainType::Evm => evm::Scanner::new(i, chain, sender.clone()).await?.run(),
            }
            tracing::info!(
                "{} scanning, main account: {}, tokens: {:?}",
                chain.chain_name,
                chain.wallet.address(),
                chain.tokens,
            );
        }

        tokio::spawn(self.listen(receiver));
        Ok(sender)
    }

    async fn listen(self, mut recv: UnboundedReceiver<ScannerMessage>) {
        loop {
            match recv.recv().await {
                Some(ScannerMessage::Deposit(index, deposit)) => match deposit {
                    ChainDeposit::Evm(token, customer, value, tx) => {
                        let _ = self
                            .handle_evm_deposit(index, token, customer, value, tx)
                            .await;
                    }
                },
                Some(ScannerMessage::Scanned(index, block)) => {
                    let _ = self
                        .storage
                        .set_scanned_block(&self.chains[index].chain_name, block)
                        .await;
                }
                None => break,
            }
        }
    }

    async fn handle_evm_deposit(
        &self,
        index: usize,
        token: Address,
        customer: Address,
        value: U256,
        tx: B256,
    ) -> Result<()> {
        // 1. check address or transaction is exists
        let cs = customer.to_checksum(None);
        let tx = format!("{:?}", tx);
        let (mid, cid, merchant) = self.storage.contains_address(&cs).await?;
        self.storage.no_transaction(&tx).await?;
        let merchant: Address = merchant.parse()?;

        // 2. save the new deposited
        let chain = &self.chains[index];
        let decimal = chain.decimals.get(&token).unwrap_or(&6);
        let identity = chain
            .tokens
            .get(&token)
            .cloned()
            .unwrap_or(chain.chain_name.clone());
        let amount = evm::u256_to_i32(value, decimal);
        let did = self
            .storage
            .deposited(identity.clone(), mid, cid, amount, tx.clone())
            .await?;

        // 2. generate customer secret key
        let (sk, _addr) = generate_eth(mid, cid, &self.mnemonics)?;
        let customer_wallet: PrivateKeySigner = sk.parse()?;

        // 3. do transfer onchain
        let (settled_amount, settled_tx) = evm::transfer(
            customer,
            merchant,
            token,
            customer_wallet,
            chain.wallet.clone(),
            chain.rpc.clone(),
            chain.commission,
            evm::i32_to_u256(chain.commission_min, decimal),
            evm::i32_to_u256(chain.commission_max, decimal),
        )
        .await
        .map_err(|err| {
            tracing::error!("TRANSFER: {tx} failed: {:?}", err);
            err
        })?;

        // 4. save the settled to deposit
        let settled_amount = evm::u256_to_i32(settled_amount, decimal);
        let settled_tx = format!("{:?}", settled_tx);
        let _ = self
            .storage
            .settled(identity, did, settled_amount, settled_tx)
            .await;

        Ok(())
    }
}

// pub async fn fetch_gas_token_price() -> Result<i32> {
//     let url = format!("https://api.coingecko.com/api/v3/simple/price?ids=name&vs_currencies=usd", name);
//     let response = reqwest::get(format!("")).await?;
//     let data = response.json()?.await?;
// }
