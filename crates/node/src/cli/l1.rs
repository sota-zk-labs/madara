use std::time::Duration;

use url::Url;

use mp_utils::parsers::{parse_duration, parse_url};

#[derive(Clone, Debug, clap::ValueEnum, PartialEq)]
pub enum L1Type {
    Ethereum,
    Aptos
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
    #[clap(long, alias = "no-l1-sync", conflicts_with = "l1_endpoint")]
    pub sync_l1_disabled: bool,

    /// The L1 rpc endpoint url for state verification.
    #[clap(long, value_parser = parse_url, value_name = "L1 RPC URL", required_unless_present = "sync_l1_disabled")]
    pub l1_endpoint: Option<Url>,

    #[clap(long, value_name = "L1 TYPE", default_value_t = L1Type::Ethereum, required_unless_present = "sync_l1_disabled")]
    pub l1_type: L1Type,

    #[clap(long, value_name = "APTOS CONTRACT ADDRESS", required_if_eq("l1_type", "aptos"))]
    pub aptos_core_contract: Option<String>,

    /// Disable the gas price sync service. The sync service is responsible to fetch the fee history from the ethereum.
    #[clap(long, alias = "no-gas-price-sync")]
    pub gas_price_sync_disabled: bool,

    /// Time in which the gas price worker will fetch the gas price.
    #[clap(
        long,
        default_value = "10s",
        value_parser = parse_duration,
    )]
    pub gas_price_poll: Duration,
}
