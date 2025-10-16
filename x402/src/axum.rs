/// POST /verify
/// Verifies a payment authorization without executing the transaction on the blockchain.
pub async fn verify() -> Response {
    //
}

/// POST /settle
/// Executes a verified payment by broadcasting the transaction to the blockchain.
pub async fn settle() -> Response {
    //
}

/// GET /supported
/// Returns the list of payment schemes and networks supported by the facilitator.
pub async fn supported() -> Response {
    //
}

/// GET /discovery/resources
pub async fn discovery() -> Response {
    //
}

