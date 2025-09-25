/**
 * SIWE (Sign-In with Ethereum) Test Example in Rust
 *
 * This example shows how to generate a SIWE message and signature
 * that can be used to test the ZeroPay service authentication.
 *
 * To run this example:
 * cargo run --example siwe_test
 */
use alloy::{
    primitives::Address,
    signers::{Signer, local::PrivateKeySigner},
};
use rand::Rng;
use reqwest;
use serde_json::{Value, json};

#[derive(Debug)]
struct TestWallet {
    signer: PrivateKeySigner,
    address: Address,
}

impl TestWallet {
    fn new() -> Self {
        // Generate a random private key for testing
        let private_key: [u8; 32] = rand::thread_rng().r#gen();
        let signer = PrivateKeySigner::from_bytes(&private_key.into()).unwrap();
        let address = signer.address();

        Self { signer, address }
    }

    async fn sign_message(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        let signature = self.signer.sign_message(message.as_bytes()).await?;
        Ok(format!("0x{}", hex::encode(signature.as_bytes())))
    }
}

async fn get_nonce() -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get("http://localhost:9000/api/nonce").await?;
    let json: Value = response.json().await?;
    Ok(json["nonce"].as_str().unwrap().to_string())
}

async fn login(message: &str, signature: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let payload = json!({
        "message": message,
        "signature": signature
    });

    let response = client
        .post("http://localhost:9000/api/login")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    Ok(response.json().await?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SIWE Test Example for ZeroPay Service");
    println!("==========================================");
    println!();

    // Step 1: Create a test wallet
    let wallet = TestWallet::new();
    println!("🔑 Test Wallet Address: {:?}", wallet.address);
    println!();

    // Step 2: Get nonce from service
    println!("📡 Getting nonce from service...");
    let nonce = match get_nonce().await {
        Ok(n) => {
            println!("📝 Received nonce: {}", n);
            n
        }
        Err(e) => {
            println!("❌ Failed to get nonce: {}", e);
            println!("💡 Make sure the ZeroPay service is running on http://localhost:9000");
            return Ok(());
        }
    };
    println!();

    // Step 3: Create SIWE message
    println!("📄 Creating SIWE message...");
    let msg = format!(
        r#"localhost:9000 wants you to sign in with your Ethereum account:
{}

Sign in to ZeroPay service

URI: http://localhost:9000
Version: 1
Chain ID: 1
Nonce: {}
Issued At: 2025-12-07T18:28:18.807Z"#,
        wallet.address.to_checksum(None),
        nonce
    );

    // Step 4: Sign the message
    println!("✍️  Signing message:\n{msg}");
    let signature = wallet.sign_message(&msg).await?;
    println!("✍️  Signature: {}", signature);
    println!();

    // Step 5: Test login
    println!("🔐 Testing login...");
    match login(&msg, &signature).await {
        Ok(result) => {
            if result.get("token").is_some() {
                println!("✅ Login successful!");
                println!("🎟️  JWT Token: {}", result["token"]);
                println!("📍 Address: {}", result["address"]);
            } else {
                println!("❌ Login failed: {}", result["error"]);
            }
        }
        Err(e) => {
            println!("❌ Login failed: {}", e);
        }
    }

    Ok(())
}
