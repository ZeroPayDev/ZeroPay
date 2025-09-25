use crate::error::Result;
use chrono::prelude::*;
use redis::Client as RedisClient;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Serialize, Deserialize)]
pub struct Customer {
    pub id: i32,
    pub merchant: i32,
    pub account: String,
    pub eth: String,
    pub updated_at: NaiveDateTime,
}

impl Customer {
    pub async fn get(id: i32, db: &PgPool) -> Result<Self> {
        let res = query_as!(Self, "SELECT * FROM customers WHERE id=$1", id)
            .fetch_one(db)
            .await?;

        Ok(res)
    }

    pub async fn get_by_eth(eth: &str, db: &PgPool) -> Result<Self> {
        let res = query_as!(Self, "SELECT * FROM customers WHERE eth=$1", eth)
            .fetch_one(db)
            .await?;

        Ok(res)
    }

    /// get customer by merchant and account
    pub async fn get_by_merchant(merchant: i32, account: &str, db: &PgPool) -> Result<Self> {
        let res = query_as!(
            Self,
            "SELECT * FROM customers WHERE merchant=$1 AND account=$2",
            merchant,
            account
        )
        .fetch_one(db)
        .await?;

        Ok(res)
    }

    /// get or insert the account by given account
    pub async fn get_or_insert(
        merchant: i32,
        account: String,
        db: &PgPool,
        mem: &str,
        redis: Option<&RedisClient>,
    ) -> Result<Self> {
        if let Ok(a) = Self::get_by_merchant(merchant, &account, db).await {
            Ok(a)
        } else {
            let now = Utc::now().naive_utc();
            let id = query_scalar!(
                "INSERT INTO customers(merchant,account,eth,updated_at) VALUES ($1,$2,$3,$4) RETURNING id",
                merchant,
                account,
                "",
                now
            )
            .fetch_one(db)
            .await?;

            let (_, eth) = crate::did::generate_eth(merchant, id, mem)?;
            // Add more accounts
            let _ = query!("UPDATE customers SET eth=$1 WHERE id=$2", eth, id)
                .execute(db)
                .await?;

            // Store the customer address in Redis for fast lookup
            if let Some(redis_client) = redis {
                if let Err(e) = Self::store_address_in_redis(redis_client, &eth).await {
                    tracing::error!("Failed to store customer address {} in Redis: {:?}", eth, e);
                    // Don't fail the entire operation if Redis fails
                }
            }

            Ok(Self {
                id,
                merchant,
                account,
                eth,
                updated_at: now,
            })
        }
    }

    /// Store customer address in Redis for fast lookup during scanning
    async fn store_address_in_redis(redis: &RedisClient, eth_address: &str) -> Result<()> {
        use redis::AsyncCommands;

        let mut conn = redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| crate::error::ApiError::Internal)?;

        let key = format!("customer_addr:{}", eth_address);
        let _: () = conn
            .set(&key, "1")
            .await
            .map_err(|_| crate::error::ApiError::Internal)?;

        // Set expiration to 30 days
        let _: bool = conn
            .expire(&key, 30 * 24 * 3600)
            .await
            .map_err(|_| crate::error::ApiError::Internal)?;

        tracing::info!("Stored customer address in Redis: {}", eth_address);
        Ok(())
    }

    /// Load all existing customer addresses into Redis on startup
    pub async fn load_all_addresses_to_redis(db: &PgPool, redis: &RedisClient) -> Result<()> {
        let addresses = query_scalar!("SELECT eth FROM customers WHERE eth != ''")
            .fetch_all(db)
            .await?;

        tracing::info!("Loading {} customer addresses to Redis", addresses.len());

        for address in addresses {
            if let Err(e) = Self::store_address_in_redis(redis, &address).await {
                tracing::error!("Failed to load address {} to Redis: {:?}", address, e);
            }
        }

        tracing::info!("Finished loading customer addresses to Redis");
        Ok(())
    }
}
