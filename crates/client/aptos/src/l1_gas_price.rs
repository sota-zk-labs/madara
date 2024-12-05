use std::time::{Duration, SystemTime, UNIX_EPOCH};
use mc_mempool::{GasPriceProvider, L1DataProvider};
use mp_utils::service::ServiceContext;
use mp_utils::wait_or_graceful_shutdown;

use crate::client::AptosClient;

pub async fn gas_price_worker_once(
    aptos_client: &AptosClient,
    l1_gas_provider: GasPriceProvider,
    gas_price_poll_ms: Duration,
) -> anyhow::Result<()> {
    match update_gas_price(aptos_client, l1_gas_provider.clone()).await {
        Ok(_) => tracing::trace!("Updated gas prices"),
        Err(e) => tracing::error!("Failed to update gas prices: {:?}", e),
    }

    let last_update_timestamp = l1_gas_provider.get_gas_prices_last_update();
    let duration_since_last_update = SystemTime::now().duration_since(last_update_timestamp)?;
    let last_update_timestemp =
        last_update_timestamp.duration_since(UNIX_EPOCH).expect("SystemTime before UNIX EPOCH!").as_micros();
    if duration_since_last_update > 10 * gas_price_poll_ms {
        anyhow::bail!(
            "Gas prices have not been updated for {} ms. Last update was at {}",
            duration_since_last_update.as_micros(),
            last_update_timestemp
        );
    }

    Ok(())
}

pub async fn gas_price_worker(
    aptos_client: &AptosClient,
    l1_gas_provider: GasPriceProvider,
    gas_price_poll_ms: Duration,
    ctx: ServiceContext,
) -> anyhow::Result<()> {
    l1_gas_provider.update_last_update_timestamp();
    let mut interval = tokio::time::interval(gas_price_poll_ms);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    while wait_or_graceful_shutdown(interval.tick(), &ctx).await.is_some() {
        gas_price_worker_once(aptos_client, l1_gas_provider.clone(), gas_price_poll_ms).await?;
    }

    Ok(())
}

async fn update_gas_price(aptos_client: &AptosClient, l1_gas_provider: GasPriceProvider) -> anyhow::Result<()> {
    let latest_block = aptos_client.provider.get_ledger_information().await?.into_inner().block_height;

    let txs = aptos_client.provider.get_block_by_height(latest_block, true).await?.into_inner().transactions.unwrap();
    let latest_gas_fee = txs.last().unwrap().transaction_info()?.gas_used.0;
    let avg_gas_fee =
        txs.clone().into_iter().map(|tx| tx.transaction_info().unwrap().gas_used.0).sum::<u64>() / txs.len() as u64;

    l1_gas_provider.update_eth_l1_gas_price(latest_gas_fee as u128);
    l1_gas_provider.update_eth_l1_data_gas_price(avg_gas_fee as u128);

    update_l1_block_metrics(aptos_client, l1_gas_provider).await?;

    Ok(())
}

async fn update_l1_block_metrics(aptos_client: &AptosClient, l1_gas_provider: GasPriceProvider) -> anyhow::Result<()> {
    let latest_block_number = aptos_client.get_latest_block_number().await?;

    // Get the current gas price
    let current_gas_price = l1_gas_provider.get_gas_prices();
    let eth_gas_price = current_gas_price.eth_l1_gas_price;

    // Update the metrics
    aptos_client.l1_block_metrics.l1_block_number.record(latest_block_number, &[]);
    aptos_client.l1_block_metrics.l1_gas_price_wei.record(eth_gas_price as u64, &[]);

    // We're ignoring l1_gas_price_strk
    Ok(())
}
