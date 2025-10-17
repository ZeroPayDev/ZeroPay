use crate::{
    Payee, PaymentRequirements, PaymentScheme, SCHEME, SettlementResponse, VerifyRequest,
    VerifyResponse,
};
use anyhow::Result;
use std::collections::HashMap;

pub struct SolScheme {
    scheme: String,
    network: String,
    rpc: String,
}

impl SolScheme {
    pub fn new(url: &str, network: &str) -> Result<Self> {
        Ok(Self {
            scheme: SCHEME.to_owned(),
            network: network.to_owned(),
            rpc: url.to_owned(),
        })
    }

    pub async fn asset(&mut self, addr: &str) -> Result<()> {
        todo!()
    }
}

impl PaymentScheme for SolScheme {
    /// The scheme of this payment scheme
    fn scheme(&self) -> &str {
        &self.scheme
    }

    /// The network of this payment scheme
    fn network(&self) -> &str {
        &self.network
    }

    /// Create a payment for the client
    fn create(&self, price: f32, payee: Payee) -> Vec<PaymentRequirements> {
        todo!()
    }

    /// The facilitator performs the following verification steps:
    /// 1. Signature Validation: Verify the EIP-712 signature is valid and properly signed by the payer
    /// 2. Balance Verification: Confirm the payer has sufficient token balance for the transfer
    /// 3. Amount Validation: Ensure the payment amount meets or exceeds the required amount
    /// 4. Time Window Check: Verify the authorization is within its valid time range
    /// 5. Parameter Matching: Confirm authorization parameters match the original payment requirements
    /// 6. Transaction Simulation: Simulate the transferWithAuthorization transaction to ensure it would succeed
    fn verify(&self, req: &VerifyRequest) -> VerifyResponse {
        todo!()
    }

    /// Settlement is performed by calling the transferWithAuthorization
    /// function on the ERC-20 contract with the signature and authorization
    /// parameters provided in the payment payload.
    fn settle(&self, req: &VerifyRequest) -> SettlementResponse {
        todo!()
    }
}
