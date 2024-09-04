use std::time::Duration;
use starknet_types_core::felt::Felt;
use dc_db::DeoxysBackend;
use dc_mempool::GasPriceProvider;
use crate::client::AptosClient;
use crate::l1_gas_price::gas_price_worker;
use crate::state_update::state_update_worker;

pub async fn l1_sync_worker(
    backend: &DeoxysBackend,
    aptos_client: &AptosClient,
    chain_id: Felt,
    l1_gas_provider: GasPriceProvider,
    gas_price_sync_disable: bool,
    gas_price_poll_ms: Duration,
) -> anyhow::Result<()> {
    tokio::try_join!(state_update_worker(backend, aptos_client, chain_id), async {
        if !gas_price_sync_disable {
            gas_price_worker(aptos_client, l1_gas_provider, gas_price_poll_ms).await?
        }
        Ok(())
    })?;

    Ok(())
}