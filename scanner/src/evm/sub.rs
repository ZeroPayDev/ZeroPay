use super::EvmSubscription;
use alloy::{
    primitives::{Address, B256, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};
use anyhow::Result;

// claim the next period subscription amount
pub async fn claim(
    id: i32,
    subscription: Address,
    main: PrivateKeySigner,
    url: Url,
) -> Result<B256> {
    let provider = ProviderBuilder::new().wallet(main).connect_http(url);
    let contract = EvmSubscription::new(subscription, provider);

    let pending = contract.claim(U256::from(id)).send().await?;
    tracing::debug!("{id}: subscription sent");

    let receipt = pending.get_receipt().await?;
    tracing::debug!("{id}: subscription claimed");

    Ok(receipt.transaction_hash)
}
