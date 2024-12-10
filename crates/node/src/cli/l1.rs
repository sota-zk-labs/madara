use std::fmt::Display;
use std::time::Duration;

use url::Url;

use mp_utils::parsers::{parse_duration, parse_url};

#[derive(Clone, Debug, clap::ValueEnum, PartialEq)]
pub enum L1Type {
    Ethereum,
    Aptos,
}

impl Display for L1Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            L1Type::Ethereum => write!(f, "ethereum"),
            L1Type::Aptos => write!(f, "aptos"),
        }
    }
}

#[derive(Clone, Debug, clap::Args)]
pub struct L1SyncParams {
    /// Disable L1 sync.
    #[clap(env = "MADARA_SYNC_L1_DISABLED", long, alias = "no-l1-sync", conflicts_with = "l1_endpoint")]
    pub sync_l1_disabled: bool,

    #[clap(long, value_name = "L1 TYPE", default_value_t = L1Type::Ethereum, required_unless_present = "sync_l1_disabled")]
    pub l1_type: L1Type,

    /// The L1 rpc endpoint url for state verification.
    #[clap(long, value_parser = parse_url, value_name = "L1 RPC URL", required_unless_present = "sync_l1_disabled")]
    pub l1_endpoint: Option<Url>,

    #[clap(long, value_name = "APTOS CONTRACT ADDRESS", required_if_eq("l1_type", "aptos"))]
    pub aptos_core_contract: Option<String>,

    /// Disable the gas price sync service. The sync service is responsible to fetch the fee history from the ethereum.
    #[clap(long, alias = "no-gas-price-sync")]
    pub gas_price_sync_disabled: bool,

    /// Fix the gas price. If the gas price is fixed it won't fetch the fee history from the ethereum.
    #[clap(env = "MADARA_GAS_PRICE", long, alias = "gas-price")]
    pub gas_price: Option<u64>,

    /// Fix the blob gas price. If the gas price is fixed it won't fetch the fee history from the ethereum.
    #[clap(env = "MADARA_DATA_GAS_PRICE", long, alias = "blob-gas-price")]
    pub blob_gas_price: Option<u64>,

    /// Fix the strk gas price. If the strk gas price is fixed it won't fetch eth <-> strk price from the oracle.
    #[clap(env = "MADARA_STRK_GAS_PRICE", long, alias = "strk-gas-price")]
    pub strk_gas_price: Option<u64>,

    /// Fix the strk blob gas price. If the strk blob gas price is fixed it won't fetch eth <-> strk price from the oracle.
    #[clap(env = "MADARA_STRK_DATA_GAS_PRICE", long, alias = "strk-blob-gas-price")]
    pub strk_blob_gas_price: Option<u64>,

    /// Time in which the gas price worker will fetch the gas price.
    #[clap(
		env = "MADARA_GAS_PRICE_POLL",
        long,
        default_value = "10s",
        value_parser = parse_duration,
    )]
    pub gas_price_poll: Duration,
}
