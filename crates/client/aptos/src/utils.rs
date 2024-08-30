use std::str::FromStr;
use std::time::SystemTime;

use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::rest_client::aptos_api_types::{EntryFunctionId, ViewRequest};
use aptos_sdk::rest_client::Client;
use aptos_sdk::transaction_builder::TransactionBuilder;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::chain_id::ChainId;
use aptos_sdk::types::transaction::SignedTransaction;
use aptos_sdk::types::transaction::TransactionPayload;
use aptos_sdk::types::LocalAccount;
use starknet_types_core::felt::Felt;

pub async fn build_transaction(
    payload: TransactionPayload,
    sender: &LocalAccount,
    chain_id: ChainId,
) -> SignedTransaction {
    let i = sender.increment_sequence_number();
    let tx = TransactionBuilder::new(
        payload,
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() + 60,
        chain_id,
    )
    .sender(sender.address())
    .sequence_number(i)
    .max_gas_amount(30000)
    .gas_unit_price(100)
    .build();
    sender.sign_transaction(tx)
}

pub async fn view_request(client: &Client, account_address: AccountAddress, request: String) -> anyhow::Result<String> {
    let request = ViewRequest {
        type_arguments: vec![],
        arguments: vec![],
        function: EntryFunctionId::from_str(
            format!("{}::{}::{}", account_address.to_string().as_str(), "starknet_validity", request.as_str()).as_str(),
        )?,
    };

    let response = client.view(&request, None).await?.into_inner().first().unwrap().to_string();
    Ok(response)
}

pub(crate) fn new_account(account_address: AccountAddress) -> LocalAccount {
    let private_key = Ed25519PrivateKey::from_encoded_string(account_address.to_string().as_str()).unwrap();
    LocalAccount::new(account_address, private_key, 0)
}

pub(crate) fn trim_hash(hash: &Felt) -> String {
    let hash_str = format!("{:#x}", hash);
    let hash_len = hash_str.len();
    let prefix = &hash_str[..6 + 2];
    let suffix = &hash_str[hash_len - 6..];
    format!("{}...{}", prefix, suffix)
}
