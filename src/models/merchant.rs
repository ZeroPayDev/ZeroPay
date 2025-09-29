use crate::error::{ApiError, Result};
use chrono::prelude::{NaiveDateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Serialize, Deserialize)]
pub struct Merchant {
    pub id: i32,
    pub account: String,
    pub name: String,
    pub apikey: String,
    pub webhook: String,
    pub eth: String,
    pub updated_at: NaiveDateTime,
}

impl Merchant {
    pub async fn get(id: i32, db: &PgPool) -> Result<Self> {
        let res = query_as!(Self, "SELECT * FROM merchants WHERE id=$1", id)
            .fetch_one(db)
            .await?;

        Ok(res)
    }

    /// get the merchant by apikey
    pub async fn get_by_apikey(apikey: &str, db: &PgPool) -> Result<Self> {
        let res = query_as!(Self, "SELECT * FROM merchants WHERE apikey=$1", apikey)
            .fetch_one(db)
            .await?;

        Ok(res)
    }

    /// insert/get new merchant
    pub async fn insert(account: String, db: &PgPool) -> Result<Self> {
        if let Ok(a) = query_as!(Self, "SELECT * FROM merchants WHERE account=$1", account)
            .fetch_one(db)
            .await
        {
            Ok(a)
        } else {
            let now = Utc::now().naive_utc();
            let name = format!("M:{}", account);
            let apikey = generate_apikey();
            let id = query_scalar!(
                "INSERT INTO merchants(account,name,apikey,webhook,eth,updated_at) VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
                account,
                name,
                apikey,
                "", // default webhook is empty
                account, // default eth receiver is the account
                now
            )
            .fetch_one(db)
            .await?;

            Ok(Merchant {
                id,
                eth: account.clone(),
                account,
                name,
                apikey,
                webhook: String::new(),
                updated_at: now,
            })
        }
    }

    pub async fn update_apikey(id: i32, db: &PgPool) -> Result<String> {
        let now = Utc::now().naive_utc();
        let apikey = generate_apikey();
        let _ = query!(
            "UPDATE merchants SET apikey=$1, updated_at=$2 WHERE id=$3",
            apikey,
            now,
            id
        )
        .execute(db)
        .await?;

        Ok(apikey)
    }

    pub async fn update_info(
        id: i32,
        name: &str,
        webhook: &str,
        eth: &str,
        db: &PgPool,
    ) -> Result<()> {
        if let Ok(_) = query_as!(Self, "SELECT * FROM merchants WHERE name = $1", name)
            .fetch_one(db)
            .await
        {
            Err(ApiError::Verify("name already exists".to_owned()))
        } else {
            let now = Utc::now().naive_utc();
            let _ = query!(
                "UPDATE merchants SET name=$1,webhook=$2,eth=$3,updated_at=$4 WHERE id=$5",
                name,
                webhook,
                eth,
                now,
                id
            )
            .execute(db)
            .await?;

            Ok(())
        }
    }
}

fn generate_apikey() -> String {
    hex::encode(rand::thread_rng().r#gen::<[u8; 16]>())
}
