use std::sync::Arc;

use crate::event_tracker::EventTracker;
use crate::utils::{new_account, view_request};
use aptos_sdk::rest_client::Client;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;
use mc_analytics::register_gauge_metric_instrument;
use opentelemetry::{global, KeyValue};
use opentelemetry::{global::Error, metrics::Gauge};
use starknet_types_core::felt::Felt;
use url::Url;

#[derive(Clone, Debug)]
pub struct L1BlockMetrics {
    pub l1_block_number: Gauge<u64>,
    pub l1_gas_price_wei: Gauge<u64>,
    pub l1_gas_price_strk: Gauge<u64>,
}

impl L1BlockMetrics {
    pub fn register() -> Result<Self, Error> {
        let common_scope_attributes = vec![KeyValue::new("crate", "L1 Block")];
        let aptos_meter = global::meter_with_version(
            "crates.l1block.opentelemetry",
            Some("0.17"),
            Some("https://opentelemetry.io/schemas/1.2.0"),
            Some(common_scope_attributes.clone()),
        );

        let l1_block_number = register_gauge_metric_instrument(
            &aptos_meter,
            "l1_block_number".to_string(),
            "Gauge for madara L1 block number".to_string(),
            "".to_string(),
        );

        let l1_gas_price_wei = register_gauge_metric_instrument(
            &aptos_meter,
            "l1_gas_price_wei".to_string(),
            "Gauge for madara L1 gas price in wei".to_string(),
            "".to_string(),
        );

        let l1_gas_price_strk = register_gauge_metric_instrument(
            &aptos_meter,
            "l1_gas_price_strk".to_string(),
            "Gauge for madara L1 gas price in strk".to_string(),
            "".to_string(),
        );

        Ok(Self { l1_block_number, l1_gas_price_wei, l1_gas_price_strk })
    }
}

pub struct AptosClient {
    pub provider: Client,
    pub l1_core_contract: Arc<LocalAccount>,
    pub l1_block_metrics: L1BlockMetrics,
    pub event_tracker: EventTracker,
}

impl Clone for AptosClient {
    fn clone(&self) -> Self {
        AptosClient {
            provider: self.provider.clone(),
            l1_core_contract: Arc::clone(&self.l1_core_contract),
            l1_block_metrics: self.l1_block_metrics.clone(),
            event_tracker: self.event_tracker.clone(),
        }
    }
}

impl AptosClient {
    pub async fn new(
        url: Url,
        l1_core_address: AccountAddress,
        l1_block_metrics: L1BlockMetrics,
    ) -> anyhow::Result<Self> {
        let provider = Client::new(url);
        let account = new_account(l1_core_address);
        let event_tracker = EventTracker::new(provider.clone(), account.address(), 3);

        Ok(Self { provider, l1_core_contract: Arc::new(account), l1_block_metrics, event_tracker })
    }

    pub async fn get_latest_block_number(&self) -> anyhow::Result<u64> {
        let block_number = self.provider.get_ledger_information().await?;
        Ok(block_number.into_inner().block_height)
    }

    #[allow(unused)]
    pub async fn get_last_event_block_number(&self) -> anyhow::Result<u64> {
        !unimplemented!();
    }

    pub async fn get_last_verified_block_number(&self) -> anyhow::Result<u64> {
        let result =
            view_request(&self.provider, self.l1_core_contract.address(), "state_block_number".parse()?).await?;
        Ok(result.parse::<u64>()?)
    }

    pub async fn get_last_state_root(&self) -> anyhow::Result<Felt> {
        let result = view_request(&self.provider, self.l1_core_contract.address(), "state_root".parse()?).await?;
        Ok(Felt::from(result.parse::<u64>()?))
    }

    pub async fn get_last_verified_block_hash(&self) -> anyhow::Result<Felt> {
        let result = view_request(&self.provider, self.l1_core_contract.address(), "state_block_hash".parse()?).await?;
        Ok(Felt::from(result.parse::<u64>()?))
    }
}
