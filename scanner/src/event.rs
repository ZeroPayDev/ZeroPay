/// main session event for webhook
pub enum ScannerEvent {
    SessionPaid(i32, String, i32),
    SessionSettled(i32, String, i32),
    UnknowPaid(String, i32),
    UnknowSettled(String, i32),
}

impl ScannerEvent {
    pub async fn send(self, url: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();

        let (event, params): (&str, Vec<serde_json::Value>) = match self {
            ScannerEvent::SessionPaid(sid, customer, amount) => (
                "session.paid",
                vec![sid.into(), customer.into(), amount.into()],
            ),
            ScannerEvent::SessionSettled(sid, customer, amount) => (
                "session.settled",
                vec![sid.into(), customer.into(), amount.into()],
            ),
            ScannerEvent::UnknowPaid(customer, amount) => {
                ("unknow.paid", vec![customer.into(), amount.into()])
            }
            ScannerEvent::UnknowSettled(customer, amount) => {
                ("unknow.settled", vec![customer.into(), amount.into()])
            }
        };

        let payload = serde_json::json!({
            "event": event,
            "params": params
        });
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("failed status code"))
        }
    }
}
