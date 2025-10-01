mod chain;
mod customer;
mod deposit;
mod session;

pub use chain::ChainBlock;
pub use customer::Customer;
pub use deposit::Deposit;
pub use session::Session;

use anyhow::Result;
use redis::{AsyncCommands, Client as RedisClient};
use sqlx::PgPool;

/// main session event for webhook
pub enum Event {
    SessionPaid(i32, String, i32),
    SessionSettled(i32, String, i32),
    UnknowPaid(String, i32),
    UnknowSettled(String, i32),
}

impl Event {
    pub async fn send(self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();

        let (event, params): (&str, Vec<serde_json::Value>) = match self {
            Event::SessionPaid(sid, customer, amount) => (
                "session.paid",
                vec![sid.into(), customer.into(), amount.into()],
            ),
            Event::SessionSettled(sid, customer, amount) => (
                "session.settled",
                vec![sid.into(), customer.into(), amount.into()],
            ),
            Event::UnknowPaid(customer, amount) => {
                ("unknow.paid", vec![customer.into(), amount.into()])
            }
            Event::UnknowSettled(customer, amount) => {
                ("unknow.settled", vec![customer.into(), amount.into()])
            }
        };

        let payload = serde_json::json!({
            "event": event,
            "params": params
        });
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed status code"))
        }
    }
}

pub struct Storage {
    pub db: PgPool,
    pub redis: RedisClient,
    pub webhook: Option<String>,
    pub wallet: String,
}

impl scanner::ScannerStorage for Storage {
    async fn get_scanned_block(&self, name: &str) -> Result<i64> {
        let block = ChainBlock::get_block(name, &self.db).await;
        Ok(block)
    }

    async fn set_scanned_block(&self, name: &str, block: i64) -> Result<()> {
        let _ = ChainBlock::insert(name, block, &self.db).await;
        Ok(())
    }

    async fn contains_address(&self, address: &str) -> Result<(i32, i32, String)> {
        let key = format!("zpc:{}", address);
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        if !conn.exists(&key).await? {
            return Err(anyhow::anyhow!("No address: {address}"));
        }

        let id: i32 = conn.get(&key).await?;
        Ok((0, id, self.wallet.clone()))
    }

    async fn no_transaction(&self, tx: &str) -> Result<()> {
        let key = format!("zpt:{}", tx);
        let mut conn = self.redis.get_multiplexed_async_connection().await?;
        if conn.exists(&key).await? {
            Err(anyhow::anyhow!("Had transaction"))
        } else {
            Ok(())
        }
    }

    async fn deposited(&self, _mid: i32, cid: i32, amount: i32, tx: String) -> Result<i32> {
        // 1. Save the deposit to the database
        let did = Deposit::insert(cid, amount, tx.clone(), &self.db)
            .await
            .unwrap_or_default();

        // 2. fetch the right session and update it
        let sessions = Session::list_unused(cid, &self.db)
            .await
            .unwrap_or_default();
        let mut used_session = None;
        for session in sessions {
            if session.amount == amount {
                let _ = session.used(did, &self.db).await;
                used_session = Some(session);
                break;
            }
        }

        // 3. webhook event callback to merchant
        if let Some(webhook) = &self.webhook
            && let Ok(customer) = Customer::get(cid, &self.db).await
        {
            if let Some(session) = &used_session {
                if Event::SessionPaid(session.id, customer.account, amount)
                    .send(webhook)
                    .await
                    .is_ok()
                {
                    let _ = session.sent(&self.db).await;
                }
            } else {
                let _ = Event::UnknowPaid(customer.account, amount)
                    .send(webhook)
                    .await;
            }
        }

        // 4. save transaction to redis
        let _ = store_transaction_in_redis(&self.redis, &tx).await;

        Ok(did)
    }

    async fn settled(&self, did: i32, amount: i32, tx: String) -> Result<()> {
        // 1. Save settled to deposit
        let _ = Deposit::settle(did, amount, tx, &self.db).await;
        let deposit = Deposit::get(did, &self.db)
            .await
            .map_err(|_| anyhow::anyhow!("Not found"))?;
        let customer = Customer::get(deposit.customer, &self.db)
            .await
            .map_err(|_| anyhow::anyhow!("Not found"))?;
        let used_session = Session::get_by_deposit(did, &self.db).await;

        // 2. webhook settled event
        if let Some(webhook) = &self.webhook {
            if let Ok(session) = &used_session {
                let _ = Event::SessionSettled(session.id, customer.account, amount)
                    .send(webhook)
                    .await;
            } else {
                let _ = Event::UnknowSettled(customer.account, amount)
                    .send(webhook)
                    .await;
            }
        }

        Ok(())
    }
}

// Store customer address in Redis for fast lookup during scanning
pub async fn store_address_in_redis(redis: &RedisClient, eth: &str, id: i32) -> Result<()> {
    let mut conn = redis.get_multiplexed_async_connection().await?;

    let key = format!("zpc:{}", eth);

    // Set expiration to 30 days
    let _: () = conn.set_ex(&key, id, 30 * 24 * 3600).await?;

    debug!("Stored customer address in Redis: {}", eth);
    Ok(())
}

// Store tranaction in Redis for avoid duplicate
async fn store_transaction_in_redis(redis: &RedisClient, tx: &str) -> Result<()> {
    let mut conn = redis.get_multiplexed_async_connection().await?;

    let key = format!("zpt:{}", tx);

    // Set expiration to 1 days enough
    let _: () = conn.set_ex(&key, 1, 24 * 3600).await?;

    debug!("Stored transaction in Redis: {}", tx);
    Ok(())
}
