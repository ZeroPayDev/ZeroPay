mod customer;
mod deposit;
mod merchant;
mod session;
// mod chain;

pub use customer::Customer;
pub use deposit::Deposit;
pub use merchant::Merchant;
pub use session::Session;
// pub use chain::Chain;

/// main session event for webhook
pub enum Event {
    SessionPaid(i32, String, i32),
    SessionSettled(i32, String, i32),
    UnknowPaid(String, i32),
    UnknowSettled(String, i32),
}

impl Event {
    pub async fn send(self, url: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();

        let (event, params): (&str, Vec<serde_json::Value>) = match self {
            Event::SessionPaid(sid, customer, amount) => (
                "session.paid",
                vec![sid.into(), customer.into(), amount.into()],
            ),
            Event::SessionSettled(sid, customer, amount) => (
                "session.settled",
                vec![sid.into(), customer.into(), amount.into()],
            ),
            Event::UnknowPaid(customer, amount) => {
                ("unknow.paid", vec![customer.into(), amount.into()])
            }
            Event::UnknowSettled(customer, amount) => {
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
