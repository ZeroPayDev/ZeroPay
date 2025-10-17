use crate::{
    Authorization, PaymentPayload, PaymentRequirements, SCHEME, SchemePayload, X402_VERSION,
};
use alloy::{
    primitives::{Address, B256, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol,
    transports::http::reqwest::Url,
};
use anyhow::Result;
use std::collections::HashMap;

/// Payment method, support evm and sol
enum PaymentMethod {
    /// rpc endpoint, wallet
    Evm(Url, PrivateKeySigner),
}

/// Main client facilitator used to sign and build payment payload
pub struct ClientFacilitator {
    methods: HashMap<String, PaymentMethod>,
}

impl ClientFacilitator {
    /// Create new facilitator
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Register new payment scheme to it
    pub fn register(&mut self, scheme: &str, network: &str, method: PaymentMethod) {
        let identity = format!("{}-{}", scheme, network);
        self.methods.insert(identity, method);
    }

    /// Build the payment payload by first matched paymentRequirements
    pub fn build<'a>(
        &self,
        prs: &'a [PaymentRequirements],
    ) -> Result<(PaymentPayload, &'a PaymentRequirements)> {
        for pr in prs.iter() {
            let identity = format!("{}-{}", pr.scheme, pr.network);
            if self.methods.contains_key(&identity) {
                let payload = self.build_with_scheme(pr)?;
                return Ok((payload, pr));
            }
        }

        Err(anyhow::anyhow!("No matched scheme and network"))
    }

    /// Build the payment payload by a paymentRequirements
    pub fn build_with_scheme(&self, pr: &PaymentRequirements) -> Result<PaymentPayload> {
        let identity = format!("{}-{}", pr.scheme, pr.network);
        if let Some(method) = self.methods.get(&identity) {
            let (signature, authorization) =
                Self::build_evm_authorization(method, &pr.maxAmountRequired, &pr.payTo)?;
            Ok(PaymentPayload {
                x402Version: X402_VERSION,
                scheme: SCHEME.to_owned(),
                network: pr.network.clone(),
                payload: SchemePayload {
                    signature,
                    authorization,
                },
            })
        } else {
            Err(anyhow::anyhow!("No this scheme and network"))
        }
    }

    fn build_evm_authorization(
        method: &PaymentMethod,
        amount: &str,
        payee: &str,
    ) -> Result<(String, Authorization)> {
        let account = "".to_owned();
        let validAfter = "".to_owned();
        let validBefore = "".to_owned();
        let nonce = "".to_owned();

        let sign = "".to_owned();
        let auth = Authorization {
            from: account,
            to: payee.to_owned(),
            value: amount.to_owned(),
            validAfter,
            validBefore,
            nonce,
        };

        Ok((sign, auth))
    }
}
