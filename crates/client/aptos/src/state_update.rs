use crate::client::{AptosClient, L1BlockMetrics};
use crate::utils::trim_hash;
use anyhow::Context;
use dc_db::DeoxysBackend;
use dp_transactions::MAIN_CHAIN_ID;
use serde::Deserialize;
use starknet_types_core::felt::Felt;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct L1StateUpdate {
    pub block_number: u64,
    pub global_root: Felt,
    pub block_hash: Felt,
}

pub async fn get_initial_state(aptos_client: &AptosClient) -> anyhow::Result<L1StateUpdate> {
    let block_number = aptos_client.get_last_verified_block_number().await?;
    let block_hash = aptos_client.get_last_verified_block_hash().await?;
    let global_root = aptos_client.get_last_state_root().await?;

    Ok(L1StateUpdate { global_root, block_number, block_hash })
}

pub async fn listen_and_update_state(
    aptos_client: &AptosClient,
    backend: &DeoxysBackend,
    block_metrics: &L1BlockMetrics,
    chain_id: Felt,
) -> anyhow::Result<()> {
    let event_filter = aptos_client
        .provider
        .get_account_events(aptos_client.l1_core_contract.address(), "LogStateUpdate", "", None, None)
        .await?
        .into_inner();

    for event in event_filter {
        log::info!("Formatting event into an L1StateUpdate");

        // TODO: Remove unwrap()
        let data = event.data;
        let block_number = data.get("block_number").unwrap().as_u64().unwrap();
        let global_root = Felt::from(data.get("global_root").unwrap().as_u64().unwrap());
        let block_hash = Felt::from(data.get("block_hash").unwrap().as_u64().unwrap());

        let format_event = L1StateUpdate { block_number, global_root, block_hash };

        update_l1(backend, format_event, block_metrics, chain_id)?
    }

    Ok(())
}

pub fn update_l1(
    backend: &DeoxysBackend,
    state_update: L1StateUpdate,
    block_metrics: &L1BlockMetrics,
    chain_id: Felt,
) -> anyhow::Result<()> {
    if state_update.block_number > 500000u64 || chain_id == MAIN_CHAIN_ID {
        log::info!(
            "🔄 Updated L1 head #{} ({}) with state root ({})",
            state_update.block_number,
            trim_hash(&state_update.block_hash),
            trim_hash(&state_update.global_root)
        );

        block_metrics.l1_block_number.set(state_update.block_number as f64);

        backend
            .write_last_confirmed_block(state_update.block_number)
            .context("Setting l1 last confirmed block number")?;
        log::debug!("update_l1: wrote last confirmed block number");
    }

    Ok(())
}

pub async fn state_update_worker(
    backend: &DeoxysBackend,
    aptos_client: &AptosClient,
    chain_id: Felt,
) -> anyhow::Result<()> {
    backend.clear_last_confirmed_block().context("Clearing l1 last confirmed block number")?;
    log::debug!("update_l1: cleared confirmed block number");

    log::info!("🚀 Subscribed to L1 state verification");
    let initial_state = get_initial_state(aptos_client).await.context("Getting initial ethereum state")?;
    update_l1(backend, initial_state, &aptos_client.l1_block_metrics, chain_id)?;

    listen_and_update_state(aptos_client, backend, &aptos_client.l1_block_metrics, chain_id)
        .await
        .context("Subscribing to the LogStateUpdate event")?;

    Ok(())
}
