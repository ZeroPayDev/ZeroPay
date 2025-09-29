mod evm;

use crate::{
    ConfigChain,
    did::generate_eth,
    error::{ApiError, Result},
    models::{Customer, Deposit, Event, Merchant, Session},
};
use alloy::{
    primitives::{Address, B256, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};
use rand::Rng;
use redis::Client as RedisClient;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::{
    select,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        oneshot::{Sender, channel},
    },
    time::interval,
};

pub enum ChainMessage {
    // chain_id, token_address, to_address, amount, tx_hash
    Deposit(usize, Address, Address, U256, B256),
    NonceClear,
    NonceGenerate(Sender<String>),
    NonceCheck(String, Sender<bool>),
}

struct ChainInfo {
    wallet: PrivateKeySigner,
    rpc: Url,
    commission: i32,
    decimals: HashMap<Address, u8>,
}

pub async fn run(
    mnemonics: String,
    db: PgPool,
    configs: Vec<ConfigChain>,
    redis: RedisClient,
) -> UnboundedSender<ChainMessage> {
    let (sender, receiver) = unbounded_channel::<ChainMessage>();

    // parse the chain configure
    let mut chains = vec![];
    let mut chain_id = 0;
    for config in configs {
        let wallet: PrivateKeySigner = config.admin.parse().unwrap();
        let rpc: Url = config.rpc.parse().unwrap();
        let provider = ProviderBuilder::new().connect_http(rpc.clone());
        let mut tokens = vec![];
        let mut decimals = HashMap::new();
        for t in config.tokens.iter() {
            let token: Address = t.parse().unwrap();
            match evm::get_token_decimal(token, provider.clone()).await {
                Ok(decimal) => {
                    tokens.push(token);
                    decimals.insert(token, decimal);
                }
                Err(_) => {
                    error!("Invalid token address: {}", t);
                }
            }
        }

        // start the chain with Redis client for address validation
        evm::Scanner::new(
            db.clone(),
            chain_id,
            rpc.clone(),
            tokens,
            sender.clone(),
            redis.clone(),
        )
        .await
        .unwrap()
        .run();

        chains.push(ChainInfo {
            wallet,
            rpc,
            decimals,
            commission: config.commission,
        });
        chain_id += 1;
    }

    tokio::spawn(listen(mnemonics, db, receiver, chains));
    sender
}

async fn listen(
    mnemonics: String,
    db: PgPool,
    mut recv: UnboundedReceiver<ChainMessage>,
    chains: Vec<ChainInfo>,
) {
    let clear_time = Duration::from_secs(600);
    let mut interval1 = interval(clear_time); // 10min clean
    let mut nonces: HashMap<String, Instant> = HashMap::new();

    loop {
        let work = select! {
            w = async {
                recv.recv().await
            } => w,
            w = async {
                interval1.tick().await;
                Some(ChainMessage::NonceClear)
            } => w,
        };

        match work {
            Some(ChainMessage::Deposit(chain_id, token_address, to_address, amount, tx_hash)) => {
                let _ = handle_deposit(
                    &chains[chain_id],
                    token_address,
                    to_address,
                    amount,
                    tx_hash,
                    &mnemonics,
                    &db,
                )
                .await
                .map_err(|err| error!("Deposit tx error: {:?}", err));
            }
            Some(ChainMessage::NonceClear) => {
                // clean up time over 10min nonce
                let now = Instant::now();
                let removals: Vec<_> = nonces
                    .iter()
                    .filter_map(|(k, v)| {
                        if now - *v > clear_time {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                for remove in removals {
                    let _ = nonces.remove(&remove);
                }
            }
            Some(ChainMessage::NonceGenerate(receipt)) => {
                let nonce = hex::encode(rand::thread_rng().r#gen::<[u8; 16]>());
                nonces.insert(nonce.clone(), Instant::now());
                let _ = receipt.send(nonce);
            }
            Some(ChainMessage::NonceCheck(nonce, receipt)) => {
                let got = nonces.remove(&nonce).is_some();
                let _ = receipt.send(got);
            }
            None => break,
        }
    }
}

async fn handle_deposit(
    chain: &ChainInfo,
    token_address: Address,
    to_address: Address,
    amount: U256,
    tx_hash: B256,
    mnemonics: &str,
    db: &PgPool,
) -> Result<()> {
    let decimal = chain.decimals.get(&token_address).unwrap_or(&6);
    let amount = evm::u256_to_i32(amount, decimal);

    // 1. get the right customer and merchant
    let addr = to_address.to_checksum(None);
    let customer = Customer::get_by_eth(&addr, db).await?;
    let merchant = Merchant::get(customer.merchant, db).await?;
    let merchant_address: Address = merchant.eth.parse().map_err(|_| ApiError::Internal)?;

    // 2. Save the deposit to the database
    let did = Deposit::insert(customer.id, amount, format!("{:?}", tx_hash), db)
        .await
        .unwrap_or_default();

    // 3. fetch the right session and update it
    let sessions = Session::list_unused(customer.id, db)
        .await
        .unwrap_or_default();
    let mut used_session = None;
    for session in sessions {
        if session.amount == amount {
            let _ = session.used(did, db).await;
            used_session = Some(session);
            break;
        }
    }

    // 4. webhook event callback to merchant
    if !merchant.webhook.is_empty() {
        if let Some(session) = &used_session {
            if Event::SessionPaid(session.id, customer.account.clone(), amount)
                .send(&merchant.webhook)
                .await
                .is_ok()
            {
                let _ = session.sent(db).await;
            }
        } else {
            let _ = Event::UnknowPaid(customer.account.clone(), amount)
                .send(&merchant.webhook)
                .await;
        }
    }

    // 5. do transfer
    let (sk, addr2) = generate_eth(merchant.id, customer.id, mnemonics)?;
    let customer_wallet: PrivateKeySigner = sk.parse().map_err(|_| ApiError::Internal)?;
    assert_eq!(addr, addr2);

    let (settled_amount, settled_tx) = evm::transfer(
        to_address,
        merchant_address,
        token_address,
        customer_wallet,
        chain.wallet.clone(),
        chain.rpc.clone(),
        chain.commission,
    )
    .await
    .map_err(|err| {
        error!("Transfer error: {:?}", err);
        ApiError::Internal
    })?;
    let settled_amount = evm::u256_to_i32(settled_amount, decimal);

    // 6. Save settled to deposit
    let _ = Deposit::settle(did, settled_amount, format!("{:?}", settled_tx), db).await;

    // 7. webhook settled event
    if !merchant.webhook.is_empty() {
        if let Some(session) = &used_session {
            let _ = Event::SessionSettled(session.id, customer.account, settled_amount)
                .send(&merchant.webhook)
                .await;
        } else {
            let _ = Event::UnknowSettled(customer.account, settled_amount)
                .send(&merchant.webhook)
                .await;
        }
    }

    Ok(())
}

/// generate new nonce for login
pub async fn get_nonce(sender: &UnboundedSender<ChainMessage>) -> Result<String> {
    let (tx, receiver) = channel();
    if sender.send(ChainMessage::NonceGenerate(tx)).is_err() {
        return Err(ApiError::Internal);
    }
    receiver.await.map_err(|_| ApiError::Internal)
}

/// check the nonce
pub async fn check_nonce(nonce: &str, sender: &UnboundedSender<ChainMessage>) -> Result<bool> {
    let (tx, receiver) = channel();
    if sender
        .send(ChainMessage::NonceCheck(nonce.to_owned(), tx))
        .is_err()
    {
        return Err(ApiError::Internal);
    }
    receiver.await.map_err(|_| ApiError::Internal)
}

// pub async fn fetch_gas_token_price() -> Result<i32> {
//     let url = format!("https://api.coingecko.com/api/v3/simple/price?ids=name&vs_currencies=usd", name);
//     let response = reqwest::get(format!("")).await?;
//     let data = response.json()?.await?;
// }
