use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use dc_db::DeoxysBackend;
use dp_transactions::MAIN_CHAIN_ID;
use serde::Deserialize;
use starknet_types_core::felt::Felt;
use tokio::time::sleep;

use crate::client::{AptosClient, L1BlockMetrics};

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
    backend: Arc<DeoxysBackend>,
    block_metrics: &L1BlockMetrics,
    chain_id: Felt,
) -> anyhow::Result<()> {
    let typ = format!("{}::starknet_validity::LogStateUpdate", aptos_client.clone().l1_core_contract.address());

    let mut event_tracker = aptos_client.clone().event_tracker;

    let block_metrics = block_metrics.clone();

    tokio::spawn(async move {
        log::info!("⭐ Looking for LogStateUpdate event!");
        loop {
            while let Some(event) = event_tracker.latest_event(typ.clone()).await {
                let block_number = event.data.get("block_number").unwrap().as_str().unwrap().parse::<u64>().unwrap();
                let global_root = Felt::from_str(event.data.get("global_root").unwrap().as_str().unwrap()).unwrap();
                let block_hash = Felt::from_str(event.data.get("block_hash").unwrap().as_str().unwrap()).unwrap();

                let format_event = L1StateUpdate { block_number, global_root, block_hash };
                update_l1(&backend, format_event, &block_metrics, chain_id).expect("TODO: panic message");
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

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
            state_update.block_hash,
            state_update.global_root,
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
    backend: Arc<DeoxysBackend>,
    aptos_client: &AptosClient,
    chain_id: Felt,
) -> anyhow::Result<()> {
    backend.clear_last_confirmed_block().context("Clearing l1 last confirmed block number")?;
    log::debug!("update_l1: cleared confirmed block number");

    log::info!("🚀 Subscribed to L1 state verification");
    let initial_state = get_initial_state(aptos_client).await.context("Getting initial aptos state")?;
    update_l1(&backend, initial_state, &aptos_client.l1_block_metrics, chain_id)?;

    listen_and_update_state(aptos_client, backend, &aptos_client.l1_block_metrics, chain_id)
        .await
        .context("Subscribing to the LogStateUpdate event")?;

    Ok(())
}
