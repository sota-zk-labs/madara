use std::str::FromStr;

use aptos_sdk::crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::crypto::ValidCryptoMaterialStringExt;
use aptos_sdk::rest_client::aptos_api_types::{EntryFunctionId, ViewRequest};
use aptos_sdk::rest_client::Client;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::LocalAccount;

pub async fn view_request(client: &Client, account_address: AccountAddress, request: String) -> anyhow::Result<String> {
    let request = ViewRequest {
        type_arguments: vec![],
        arguments: vec![],
        function: EntryFunctionId::from_str(
            format!("{}::{}::{}", account_address.to_string().as_str(), "starknet_validity", request.as_str()).as_str(),
        )?,
    };

    let response =
        client.view(&request, None).await?.into_inner().to_vec().first().unwrap().as_str().unwrap().to_string();
    Ok(response)
}

pub(crate) fn new_account(account_address: AccountAddress) -> LocalAccount {
    let private_key = Ed25519PrivateKey::from_encoded_string(account_address.to_string().as_str()).unwrap();
    LocalAccount::new(account_address, private_key, 0)
}
