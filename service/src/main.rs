#[macro_use]
extern crate tracing;

#[macro_use]
extern crate sqlx;

mod auth;
mod chain;
mod controllers;
mod did;
mod error;
mod models;

use axum::{
    Router,
    http::Method,
    routing::{get, post},
};
use clap::Parser;
use controllers::*;
use redis::Client as RedisClient;
use serde::Deserialize;
use sqlx::{
    any::Any as SqlxAny,
    migrate::MigrateDatabase,
    postgres::{PgPool, PgPoolOptions},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc::UnboundedSender};
use tower_http::cors::{Any, CorsLayer};
use tracing::level_filters::LevelFilter;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Command {
    /// Service port
    #[arg(long, env = "PORT", default_value_t = 9000)]
    port: u16,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    database: String,

    /// Redis URL
    #[arg(long, env = "REDIS_URL", default_value = "redis://127.0.0.1:6379")]
    redis: String,

    /// Secret for JWT
    #[arg(short, long, env = "SECRET", default_value = "")]
    secret: String,

    /// Website domain for session url
    #[arg(short, long, env = "DOMAIN", default_value = "http://127.0.0.1:9000")]
    domain: String,

    /// Chain configure file path
    #[arg(long, env = "CHAIN_CONFIG", default_value = "config.toml")]
    chain_config: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    mnemonics: String,
    chains: Vec<ConfigChain>,
}

#[derive(Debug, Deserialize)]
struct ConfigChain {
    admin: String,
    rpc: String,
    commission: i32,
    tokens: Vec<String>,
}

#[derive(Clone)]
struct AppState {
    db: PgPool,
    redis: RedisClient,
    mnemonics: String,
    secret: [u8; 32],
    domain: String,
    sender: UnboundedSender<chain::ChainMessage>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();
    sqlx::any::install_default_drivers();

    let args = Command::parse();
    let chain_str = std::fs::read_to_string(&args.chain_config).unwrap();
    let chain_config: Config = toml::from_str(&chain_str).unwrap();
    let Config { mnemonics, chains } = chain_config;

    // domain check and filer
    let domain = args.domain.trim_end_matches("/").to_owned();

    // setup database & init
    let _ = SqlxAny::create_database(&args.database).await;
    let db = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&args.database)
        .await
    {
        Ok(pool) => {
            info!("✅ Connection to the database is successful!");
            pool
        }
        Err(err) => {
            error!("🔥 Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    migrate!().run(&db).await.expect("Migrations failed");

    // setup redis connection
    let redis = match RedisClient::open(args.redis.clone()) {
        Ok(client) => {
            info!("✅ Redis connection established!");
            client
        }
        Err(err) => {
            error!("🔥 Failed to connect to Redis: {:?}", err);
            std::process::exit(1);
        }
    };

    // Load existing customer addresses into Redis
    if let Err(e) = models::Customer::load_all_addresses_to_redis(&db, &redis).await {
        tracing::error!("Failed to load customer addresses to Redis: {:?}", e);
        // Don't exit - this is not critical for service startup
    }

    // running listening chain & tokens
    let sender = chain::run(mnemonics.clone(), db.clone(), chains, redis.clone()).await;

    let secret_key = blake3::hash(args.secret.as_bytes());
    let secret: [u8; 32] = secret_key.into();

    let app_state = Arc::new(AppState {
        sender,
        db,
        redis,
        secret,
        mnemonics,
        domain,
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);

    let router = Router::new()
        .route("/api/nonce", get(nonce))
        .route("/api/login", post(login))
        .route("/api/sessions", post(create_session))
        .route("/api/sessions/{id}", get(get_session))
        .route("/api/merchants/info", post(update_info))
        .route("/api/merchants/apikey", post(update_apikey))
        .with_state(app_state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = TcpListener::bind(&addr).await.unwrap();
    info!("🚀 Server is running on 0.0.0.0:{}", args.port);

    axum::serve(listener, router).await.unwrap()
}
