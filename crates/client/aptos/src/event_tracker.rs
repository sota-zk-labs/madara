use aptos_sdk::rest_client::aptos_api_types::VersionedEvent;
use aptos_sdk::rest_client::Client;
use aptos_sdk::types::account_address::AccountAddress;

#[derive(Clone)]
pub struct EventTracker {
    client: Client,
    account_address: AccountAddress,
    creation_number: u64,
}

impl EventTracker {
    pub fn new(client: Client, account_address: AccountAddress, creation_number: u64) -> Self {
        Self { client, account_address, creation_number }
    }
}
impl EventTracker {
    pub async fn latest_event(&mut self, typ: String) -> Option<VersionedEvent> {
        let mut result = None;
        loop {
            let creation_number = self.creation_number + 1;
            let events = self
                .client
                .get_account_events(self.account_address, creation_number.to_string().as_str(), "", None, None)
                .await
                .unwrap()
                .into_inner();
            if events.is_empty() {
                break;
            }
            self.creation_number = creation_number;
            events.into_iter().for_each(|e| {
                if e.typ.to_string() == typ {
                    result = Some(e);
                }
            });
        }
        result
    }
}
