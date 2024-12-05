use mc_db::MadaraBackend;
use mc_mempool::GasPriceProvider;
use starknet_types_core::felt::Felt;
use std::sync::Arc;
use std::time::Duration;
use mp_utils::service::ServiceContext;
use crate::client::AptosClient;
use crate::l1_gas_price::gas_price_worker;
use crate::state_update::state_update_worker;

pub async fn l1_sync_worker(
    backend: Arc<MadaraBackend>,
    aptos_client: &AptosClient,
    chain_id: Felt,
    l1_gas_provider: GasPriceProvider,
    gas_price_sync_disable: bool,
    gas_price_poll_ms: Duration,
    ctx: ServiceContext,
) -> anyhow::Result<()> {
    tokio::try_join!(state_update_worker(backend, aptos_client, chain_id), async {
        if !gas_price_sync_disable {
            gas_price_worker(aptos_client, l1_gas_provider, gas_price_poll_ms, ctx.clone()).await?
        }
        Ok(())
    })?;

    Ok(())
}
