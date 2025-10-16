/// When a resource server requires payment, it responds with a payment required signal and a JSON payload containing payment requirements
pub struct PaymentRequirementsResponse {
    /// Protocol version identifier
    pub x402Version: i32,
    /// Human-readable error message explaining why payment is required
    pub error: String,
    /// Array of payment requirement objects defining acceptable payment methods
    pub accepts: Vec<PaymentRequirements>,
}

/// Payment requirement objects defining acceptable payment methods
pub struct PaymentRequirements {
    /// Payment scheme identifier (e.g., "exact")
    pub scheme: String,
    /// Blockchain network identifier (e.g., "base-sepolia", "ethereum-mainnet")
    pub network: String,
    /// Required payment amount in atomic token units
    pub maxAmountRequired: String,
    /// Token contract address
    pub asset: String,
    /// Recipient wallet address for the payment
    pub payTo: String,
    /// URL of the protected resource
    pub resource: String,
    /// Human-readable description of the resource
    pub description: String,
    /// MIME type of the expected response
    pub mimeType: Option<String>,
    /// JSON schema describing the response format
    pub outputSchema: Option<Value>,
    /// Maximum time allowed for payment completion
    pub maxTimeoutSeconds: i32,
    /// Scheme-specific additional information
    pub extra: Option<Value>,
}

/// The client includes payment authorization as JSON in the payment payload field
pub struct PaymentPayload {
    /// Protocol version identifier (must be 1)
    pub x402Version: i32,
    /// Payment scheme identifier (e.g., "exact")
    pub scheme: String,
    /// Blockchain network identifier (e.g., "base-sepolia", "ethereum-mainnet")
    pub network: String,
    /// Payment data object
    pub payload: SchemePayload,
}

/// Payment authorization scheme-specific data
pub struct SchemePayload {
    /// EIP-712 signature for authorization
    pub signature: String,
    /// EIP-3009 authorization parameters
    pub authorization: Authorization,
}

/// EIP-3009 authorization parameters
pub struct Authorization {
    /// Payer's wallet address
    pub from: String,
    /// Recipient's wallet address
    pub to: String,
    /// Payment amount in atomic units
    pub value: String,
    /// Unix timestamp when authorization becomes valid
    pub validAfter: String,
    /// Unix timestamp when authorization expires
    pub validBefore: String,
    /// 32-byte random nonce to prevent replay attacks
    pub nonce: String,
}

/// After payment settlement, the server includes transaction details in the payment response field as JSON
pub struct SettlementResponse {
    /// Indicates whether the payment settlement was successful
    pub success: bool,
    /// Error reason if settlement failed (omitted if successful)
    pub errorReason: Option<String>,
    /// Blockchain transaction hash (empty string if settlement failed)
    pub transaction: String,
    ///	Blockchain network identifier
    pub network: String,
    /// Address of the payer's wallet
    pub payer: String,
}

pub trait PaymentScheme {
    /// The facilitator performs the following verification steps:
    /// 1. Signature Validation: Verify the EIP-712 signature is valid and properly signed by the payer
    /// 2. Balance Verification: Confirm the payer has sufficient token balance for the transfer
    /// 3. Amount Validation: Ensure the payment amount meets or exceeds the required amount
    /// 4. Time Window Check: Verify the authorization is within its valid time range
    /// 5. Parameter Matching: Confirm authorization parameters match the original payment requirements
    /// 6. Transaction Simulation: Simulate the transferWithAuthorization transaction to ensure it would succeed
    fn verify() -> bool;

    /// Settlement is performed by calling the transferWithAuthorization function on the ERC-20 contract with the signature and authorization parameters provided in the payment payload.
    fn settle() -> bool;
}

pub struct VerifyRequest {
    pub paymentPayload: PaymentPayload,
    pub paymentRequirements: PaymentRequirements,
}

pub struct VerifyResponse {
    isValid: bool,
    payer: String,
    invalidReason: Option<String>,
}

impl VerifyRequest {
    pub fn verify(&self) -> VerifyResponse {
        //
    }

    pub fn settle(&self) -> SettlementResponse {
        //
    }
}

pub struct SupportedResponse {
    kinds: Vec<SupportedScheme>,
}

pub struct SupportedScheme {
    x402Version: i32,
    scheme: String,
    network: String,
}

/// List discoverable x402 resources from the Bazaar.
pub struct DiscoveryRequest {
    /// Filter by resource type (e.g., "http"), default is none
    r#type: Option<String>,
    /// Maximum number of results to return (1-100), default is 20
    limit: Option<i32>,
    /// Number of results to skip for pagination, default is 0
    offset: Option<i32>,
}

/// The response of discoverable resources
pub struct DiscoveryResponse {
    /// Protocol version supported by the resource
    x402Version: i32,
    /// The list of supported resources item
    items: Vec<DiscoveryItem>,
    /// Pagination
    pagination: Pagination,
}

/// Discoverable resources item
pub struct DiscoveryItem {
    /// The resource URL or identifier being monetized
    resource: String,
    /// Resource type (currently "http" for HTTP endpoints)
    r#type: String,
    /// Protocol version supported by the resource
    x402Version: i32,
    /// Array of PaymentRequirements objects specifying payment methods
    accepts: Vec<PaymentRequirements>,
    /// Unix timestamp of when the resource was last updated
    lastUpdated: i64,
    /// Additional metadata (category, provider, etc.)
    metadata: Option<Value>,
}

/// Pagination for discovery
pub struct Pagination {
    /// The number of items in a response
    limit: i32,
    /// The start point of this query
    offset: i32,
    /// The total number of all items
    total: i32,
}

/// The error
pub enum Error {
    /// Client does not have enough tokens to complete the payment
    InsufficientFunds,
    /// Payment authorization is not yet valid (before validAfter timestamp)
    InvalidExactEvmPayloadAuthorizationValidAfter,
    /// Payment authorization has expired (after validBefore timestamp)
    InvalidExactEvmPayloadAuthorizationValidBefore,
    /// Payment amount is insufficient for the required payment
    InvalidExactEvmPayloadAuthorizationValue,
    /// Payment authorization signature is invalid or improperly signed
    InvalidExactEvmPayloadSignature,
    /// Recipient address does not match payment requirements
    InvalidExactEvmPayloadRecipientMismatch,
    /// Specified blockchain network is not supported
    InvalidNetwork,
    /// Payment payload is malformed or contains invalid data
    InvalidPayload,
    /// Payment requirements object is invalid or malformed
    InvalidPaymentRequirements,
    /// Specified payment scheme is not supported
    InvalidScheme,
    /// Payment scheme is not supported by the facilitator
    UnsupportedScheme,
    /// Protocol version is not supported
    InvalidX402Version,
    /// Blockchain transaction failed or was rejected
    InvalidTransactionState,
    /// Unexpected error occurred during payment verification
    UnexpectedVerifyError,
    /// Unexpected error occurred during payment settlement
    UnexpectedSettleError,
}

impl Error {
    pub fn to_code(&self) -> (&str, &str) {
        match self {
            Error::InsufficientFunds => (
                "insufficient_funds",
                "Client does not have enough tokens to complete the payment",
            ),
            Error::InvalidExactEvmPayloadAuthorizationValidAfter => (
                "invalid_exact_evm_payload_authorization_valid_after",
                "Payment authorization is not yet valid (before validAfter timestamp)",
            ),
            Error::InvalidExactEvmPayloadAuthorizationValidBefore => (
                "invalid_exact_evm_payload_authorization_valid_before",
                "Payment authorization has expired (after validBefore timestamp)",
            ),
            Error::InvalidExactEvmPayloadAuthorizationValue => (
                "invalid_exact_evm_payload_authorization_value",
                "Payment amount is insufficient for the required payment",
            ),
            Error::InvalidExactEvmPayloadSignature => (
                "invalid_exact_evm_payload_signature",
                "Payment authorization signature is invalid or improperly signed",
            ),
            Error::InvalidExactEvmPayloadRecipientMismatch => (
                "invalid_exact_evm_payload_recipient_mismatch",
                "Recipient address does not match payment requirements",
            ),
            Error::InvalidNetwork => (
                "invalid_network",
                "Specified blockchain network is not supported",
            ),
            Error::InvalidPayload => (
                "invalid_payload",
                "Payment payload is malformed or contains invalid data",
            ),
            Error::InvalidPaymentRequirements => (
                "invalid_payment_requirements",
                "Payment requirements object is invalid or malformed",
            ),
            Error::InvalidScheme => (
                "invalid_scheme",
                "Specified payment scheme is not supported",
            ),
            Error::UnsupportedScheme => (
                "unsupported_scheme",
                "Payment scheme is not supported by the facilitator",
            ),
            Error::InvalidX402Version => {
                ("invalid_x402_version", "Protocol version is not supported")
            }
            Error::InvalidTransactionState => (
                "invalid_transaction_state",
                "Blockchain transaction failed or was rejected",
            ),
            Error::UnexpectedVerifyError => (
                "unexpected_verify_error",
                "Unexpected error occurred during payment verification",
            ),
            Error::UnexpectedSettleError => (
                "unexpected_settle_error",
                "Unexpected error occurred during payment settlement",
            ),
        }
    }
}
