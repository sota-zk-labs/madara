#[allow(dead_code)]

use alloy::primitives::FixedBytes;
use starknet_api::core::ChainId;
use starknet_types_core::felt::Felt;
use dc_db::DeoxysBackend;
use crate::client::AptosClient;

impl AptosClient {
    pub async fn get_l1_to_l2_message_cancellations(&self, msg_hash: FixedBytes<32>) -> anyhow::Result<Felt> {
        !unimplemented!()
    }
}

pub async fn sync(backend: &DeoxysBackend, aptos_client: &AptosClient, chain_id: &ChainId) -> anyhow::Result<()> {
    !unimplemented!()
}

// async fn process_l1_message(backend: &DeoxysBackend, event: &LogMessageToL2, l1_block_number: &Option<u64>, event_index: &Option<u64>, chain_id: &ChainId) -> anyhow::Result<>