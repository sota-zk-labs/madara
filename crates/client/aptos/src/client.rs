use std::sync::Arc;

use aptos_sdk::rest_client::Client;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;
use dc_metrics::{Gauge, MetricsRegistry, PrometheusError, F64};
use starknet_types_core::felt::Felt;
use url::Url;

use crate::utils::{new_account, view_request};

#[derive(Clone, Debug)]
pub struct L1BlockMetrics {
    pub l1_block_number: Gauge<F64>,
    pub l1_gas_price_wei: Gauge<F64>,
    pub l1_gas_price_strk: Gauge<F64>,
}

impl L1BlockMetrics {
    pub fn register(registry: &MetricsRegistry) -> Result<Self, PrometheusError> {
        Ok(Self {
            l1_block_number: registry
                .register(Gauge::new("deoxys_l1_block_number", "Gauge for deoxys L1 block number")?)?,

            l1_gas_price_wei: registry.register(Gauge::new("deoxys_l1_gas_price", "Gauge for deoxys L1 gas price")?)?,
            l1_gas_price_strk: registry
                .register(Gauge::new("deoxys_l1_gas_price_strk", "Gauge for deoxys L1 gas price in strk")?)?,
        })
    }
}

pub struct AptosClient {
    pub provider: Client,
    pub l1_core_contract: Arc<LocalAccount>,
    pub l1_block_metrics: L1BlockMetrics,
}

impl Clone for AptosClient {
    fn clone(&self) -> Self {
        AptosClient {
            provider: self.provider.clone(),
            l1_core_contract: Arc::clone(&self.l1_core_contract),
            l1_block_metrics: self.l1_block_metrics.clone(),
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

        Ok(Self { provider, l1_core_contract: Arc::new(account), l1_block_metrics })
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
        let result = view_request(&self.provider, self.l1_core_contract.address(), "state_block_root".parse()?).await?;
        Ok(Felt::from(result.parse::<u64>()?))
    }

    pub async fn get_last_verified_block_hash(&self) -> anyhow::Result<Felt> {
        let result = view_request(&self.provider, self.l1_core_contract.address(), "state_block_hash".parse()?).await?;
        Ok(Felt::from(result.parse::<u64>()?))
    }
}
