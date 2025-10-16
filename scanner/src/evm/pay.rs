use super::EvmToken;
use alloy::{
    network::TransactionBuilder,
    primitives::{Address, B256, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};
use anyhow::Result;

// transfer token from deposit to admin, return real merchant amount
#[allow(clippy::too_many_arguments)]
pub async fn transfer(
    customer: Address,
    merchant: Address,
    token: Address,
    wallet: PrivateKeySigner,
    main: PrivateKeySigner,
    url: Url,
    commission_rate: i32,
    commission_min: U256,
    commission_max: U256,
) -> Result<(U256, B256)> {
    let zero = U256::from(0);
    let maccount = main.address();
    let provider = ProviderBuilder::new()
        .wallet(main)
        .connect_http(url.clone());
    let gas_price = provider.get_gas_price().await? * 105 / 100; // add 5%
    let contract = EvmToken::new(token, provider.clone());

    // 1. check token balance
    let balance: U256 = contract.balanceOf(customer).call().await?;

    if balance == zero {
        return Err(anyhow::anyhow!("No balance"));
    }

    // 3. check approve or not
    let approved: U256 = contract.allowance(customer, maccount).call().await?;
    let need_approve = approved < balance;

    // 2. collect gas used, and do a discount in the amount
    let approve_gas = if need_approve {
        let gas = contract
            .approve(maccount, U256::from(100000000_000000i64))
            .gas_price(gas_price)
            .estimate_gas()
            .await?;
        // add more 5%
        U256::from(gas * 105 / 100) * U256::from(gas_price)
    } else {
        zero
    };
    tracing::debug!("{customer}: approve_gas: {approve_gas}");

    let fee = if commission_rate > 0 {
        let rate = balance * U256::from(commission_rate) / U256::from(100);
        let rate_max = core::cmp::min(rate, commission_max);
        core::cmp::max(rate_max, commission_min)
    } else {
        zero
    };
    let real = balance - fee;
    tracing::info!("{customer}: commission: {fee}, real: {real}");

    if need_approve {
        // 4. if not approve, transfer approve gas to it
        let ttx = TransactionRequest::default()
            .with_to(customer)
            .with_value(approve_gas);
        let pending = provider.send_transaction(ttx).await?;
        tracing::debug!("{customer}: approve gas sent");
        let _receipt = pending.get_receipt().await?;
        tracing::debug!("{customer}: approve gas arrived");

        // 5. approve tokens to max
        let customer_provider = ProviderBuilder::new().wallet(wallet).connect_http(url);
        let customer_contract = EvmToken::new(token, customer_provider);
        let total = customer_contract
            .totalSupply()
            .call()
            .await
            .unwrap_or(U256::from(100000000_000000i64));

        let pending = customer_contract
            .approve(maccount, total)
            .gas_price(gas_price)
            .send()
            .await?;
        tracing::debug!("{customer}: approved sent");
        let _receipt = pending.get_receipt().await?;
        tracing::debug!("{customer}: approved arrived");
    }

    // 6. transfer remain token to merchant
    let pending = contract
        .transferFrom(customer, merchant, real)
        .gas_price(gas_price)
        .send()
        .await?;
    tracing::debug!("{customer}: transfer real sent");
    let receipt = pending.get_receipt().await?;
    tracing::debug!("{customer}: transfer real arrived");

    if fee > zero {
        let pending2 = contract
            .transferFrom(customer, maccount, fee)
            .gas_price(gas_price)
            .send()
            .await?;
        tracing::debug!("{customer}: transfer commission sent");
        let _ = pending2.get_receipt().await?;
        tracing::debug!("{customer}: transfer commission arrived");
    }

    Ok((real, receipt.transaction_hash))
}
