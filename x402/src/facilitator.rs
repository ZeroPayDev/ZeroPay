use crate::{
    Error, Payee, PaymentRequirementsResponse, PaymentScheme,
    SettlementResponse, VerifyRequest, VerifyResponse, X402_VERSION,
};
use std::collections::HashMap;

/// The main facilitator for all payment scheme
pub struct Facilitator {
    schemes: HashMap<String, Box<dyn PaymentScheme>>,
}

impl Facilitator {
    /// Create new facilitator
    pub fn new() -> Self {
        Self {
            schemes: HashMap::new(),
        }
    }

    /// Register new payment scheme to it
    pub fn register<T: PaymentScheme + 'static>(&mut self, scheme: T) {
        let identity = scheme.identity();
        self.schemes.insert(identity, Box::new(scheme));
    }

    /// Create a payment for the client
    pub fn create(&self, price: &str, payee: Payee) -> PaymentRequirementsResponse {
        let mut payments = Vec::new();
        for (_, scheme) in self.schemes.iter() {
            payments.extend(scheme.create(price, payee.clone()));
        }

        PaymentRequirementsResponse {
            x402_version: X402_VERSION.to_owned(),
            error: "".to_owned(),
            accepts: payments,
        }
    }

    /// Verify the payment request
    pub async fn verify(&self, req: &VerifyRequest) -> VerifyResponse {
        let identity = format!(
            "{}-{}",
            req.payment_payload.scheme, req.payment_payload.network
        );
        if let Some(scheme) = self.schemes.get(&identity) {
            scheme.verify(req).await
        } else {
            VerifyResponse {
                is_valid: false,
                invalid_reason: Some(Error::UnsupportedScheme.to_code().0.to_owned()),
                payer: req.payment_payload.payload.authorization.from.clone(),
            }
        }
    }

    /// Settle the payment request
    pub async fn settle(&self, req: &VerifyRequest) -> SettlementResponse {
        let identity = format!(
            "{}-{}",
            req.payment_payload.scheme, req.payment_payload.network
        );
        if let Some(scheme) = self.schemes.get(&identity) {
            scheme.settle(req).await
        } else {
            SettlementResponse {
                success: false,
                error_reason: Some(Error::UnsupportedScheme.to_code().0.to_owned()),
                transaction: "".to_owned(),
                network: req.payment_payload.network.clone(),
                payer: req.payment_payload.payload.authorization.from.clone(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EvmScheme, SolScheme};

    #[test]
    fn test() {
        let mut registry = Facilitator::new();
        registry.register(EvmScheme::new("https://x.com", "network").unwrap());
        registry.register(SolScheme::new("rpc", "network").unwrap());
    }
}
