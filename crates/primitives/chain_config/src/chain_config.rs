// Note: We are NOT using fs read for constants, as they NEED to be included in the resulting
// binary. Otherwise, using the madara binary without cloning the repo WILL crash, and that's very very bad.
// The binary needs to be self contained! We need to be able to ship madara as a single binary, without
// the user needing to clone the repo.
// Only use `fs` for constants when writing tests.
use std::str::FromStr;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::Path,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use blockifier::bouncer::{BouncerWeights, BuiltinCount};
use blockifier::{bouncer::BouncerConfig, versioned_constants::VersionedConstants};
use lazy_static::__Deref;
use primitive_types::H160;
use serde::{Deserialize, Deserializer};
use starknet_api::core::{ChainId, ContractAddress, PatriciaKey};
use starknet_types_core::felt::Felt;

use mp_utils::serde::deserialize_duration;

use crate::StarknetVersion;

pub mod eth_core_contract_address {
    pub const MAINNET: &str = "0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4";
    pub const SEPOLIA_TESTNET: &str = "0xE2Bb56ee936fd6433DC0F6e7e3b8365C906AA057";
    pub const SEPOLIA_INTEGRATION: &str = "0x4737c0c1B4D5b1A687B42610DdabEE781152359c";
}

const BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_0: &[u8] = include_bytes!("../resources/versioned_constants_13_0.json");
const BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_1: &[u8] = include_bytes!("../resources/versioned_constants_13_1.json");
const BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_1_1: &[u8] =
    include_bytes!("../resources/versioned_constants_13_1_1.json");
const BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_2: &[u8] = include_bytes!("../resources/versioned_constants_13_2.json");

lazy_static::lazy_static! {
    pub static ref BLOCKIFIER_VERSIONED_CONSTANTS_0_13_2: VersionedConstants =
        serde_json::from_slice(BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_2).unwrap();
    pub static ref BLOCKIFIER_VERSIONED_CONSTANTS_0_13_1_1: VersionedConstants =
        serde_json::from_slice(BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_1_1).unwrap();
    pub static ref BLOCKIFIER_VERSIONED_CONSTANTS_0_13_1: VersionedConstants =
        serde_json::from_slice(BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_1).unwrap();
    pub static ref BLOCKIFIER_VERSIONED_CONSTANTS_0_13_0: VersionedConstants =
        serde_json::from_slice(BLOCKIFIER_VERSIONED_CONSTANTS_JSON_0_13_0).unwrap();
}

#[derive(thiserror::Error, Debug)]
#[error("Unsupported protocol version: {0}")]
pub struct UnsupportedProtocolVersion(StarknetVersion);

#[derive(Debug, Deserialize)]
pub struct ChainConfig {
    /// Human readable chain name, for displaying to the console.
    pub chain_name: String,
    pub chain_id: ChainId,

    /// For starknet, this is the STRK ERC-20 contract on starknet.
    pub native_fee_token_address: ContractAddress,
    /// For starknet, this is the ETH ERC-20 contract on starknet.
    pub parent_fee_token_address: ContractAddress,

    /// BTreeMap ensures order.
    pub versioned_constants: ChainVersionedConstants,

    #[serde(deserialize_with = "deserialize_starknet_version")]
    pub latest_protocol_version: StarknetVersion,

    /// Only used for block production.
    #[serde(deserialize_with = "deserialize_duration")]
    pub block_time: Duration,

    /// Only used for block production.
    /// Block time is divided into "ticks": everytime this duration elapses, the pending block is updated.  
    #[serde(deserialize_with = "deserialize_duration")]
    pub pending_block_update_time: Duration,

    /// Only used for block production.
    /// Block production is handled in batches; each batch will pop this number of transactions from the mempool. This is
    /// primarily useful for optimistic parallelization.
    /// A value too high may have a performance impact - you will need some testing to find the best value for your network.
    pub execution_batch_size: usize,

    /// Only used for block production.
    /// The bouncer is in charge of limiting block sizes. This is where the max number of step per block, gas etc are.
    #[serde(deserialize_with = "deserialize_bouncer_config")]
    pub bouncer_config: BouncerConfig,

    /// Only used for block production.
    pub sequencer_address: ContractAddress,

    /// Only used when mempool is enabled.
    /// When deploying an account and invoking a contract at the same time, we want to skip the validation step for the invoke tx.
    /// This number is the maximum nonce the invoke tx can have to qualify for the validation skip.
    pub max_nonce_for_validation_skip: u64,

    /// The Starknet core contract address for the L1 watcher.
    pub eth_core_contract_address: H160,
}

impl ChainConfig {
    pub fn from_yaml(path: &Path) -> anyhow::Result<Self> {
        let config_str = fs::read_to_string(path)?;
        serde_yaml::from_str(&config_str).context("While deserializing chain config")
    }

    /// Verify that the chain config is valid for block production.
    pub fn precheck_block_production(&self) -> anyhow::Result<()> {
        // block_time != 0 implies that n_pending_ticks_per_block != 0.
        if self.sequencer_address == ContractAddress::default() {
            bail!("Sequencer address cannot be 0x0 for block production.")
        }
        if self.block_time.as_millis() == 0 {
            bail!("Block time cannot be zero for block production.")
        }
        if self.pending_block_update_time.as_millis() == 0 {
            bail!("Block time cannot be zero for block production.")
        }
        Ok(())
    }

    pub fn starknet_mainnet() -> Self {
        // Sources:
        // - https://docs.starknet.io/tools/important-addresses
        // - https://docs.starknet.io/tools/limits-and-triggers (bouncer & block times)
        // - state_diff_size is the blob size limit of ethereum
        // - pending_block_update_time: educated guess
        // - bouncer builtin_count, message_segment_length, n_events, state_diff_size are probably wrong
        Self {
            chain_name: "Starknet Mainnet".into(),
            chain_id: ChainId::Mainnet,
            native_fee_token_address: ContractAddress(
                PatriciaKey::try_from(Felt::from_hex_unchecked(
                    "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d",
                ))
                .unwrap(),
            ),
            parent_fee_token_address: ContractAddress(
                PatriciaKey::try_from(Felt::from_hex_unchecked(
                    "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
                ))
                .unwrap(),
            ),
            versioned_constants: [
                (StarknetVersion::V0_13_0, BLOCKIFIER_VERSIONED_CONSTANTS_0_13_0.deref().clone()),
                (StarknetVersion::V0_13_1, BLOCKIFIER_VERSIONED_CONSTANTS_0_13_1.deref().clone()),
                (StarknetVersion::V0_13_1_1, BLOCKIFIER_VERSIONED_CONSTANTS_0_13_1_1.deref().clone()),
                (StarknetVersion::V0_13_2, VersionedConstants::latest_constants().clone()),
            ]
            .into(),

            eth_core_contract_address: eth_core_contract_address::MAINNET.parse().expect("parsing a constant"),

            latest_protocol_version: StarknetVersion::V0_13_2,
            block_time: Duration::from_secs(30),
            pending_block_update_time: Duration::from_secs(2),

            execution_batch_size: 16,

            bouncer_config: BouncerConfig {
                block_max_capacity: BouncerWeights {
                    builtin_count: BuiltinCount {
                        add_mod: usize::MAX,
                        bitwise: usize::MAX,
                        ecdsa: usize::MAX,
                        ec_op: usize::MAX,
                        keccak: usize::MAX,
                        mul_mod: usize::MAX,
                        pedersen: usize::MAX,
                        poseidon: usize::MAX,
                        range_check: usize::MAX,
                        range_check96: usize::MAX,
                    },
                    gas: 5_000_000,
                    n_steps: 40_000_000,
                    message_segment_length: usize::MAX,
                    n_events: usize::MAX,
                    state_diff_size: 131072,
                },
            },
            // We are not producing blocks for these chains.
            sequencer_address: ContractAddress::default(),
            max_nonce_for_validation_skip: 2,
        }
    }

    pub fn starknet_sepolia() -> Self {
        Self {
            chain_name: "Starknet Sepolia".into(),
            chain_id: ChainId::Sepolia,
            eth_core_contract_address: eth_core_contract_address::SEPOLIA_TESTNET.parse().expect("parsing a constant"),
            ..Self::starknet_mainnet()
        }
    }

    pub fn starknet_integration() -> Self {
        Self {
            chain_name: "Starknet Sepolia Integration".into(),
            chain_id: ChainId::IntegrationSepolia,
            eth_core_contract_address: eth_core_contract_address::SEPOLIA_INTEGRATION
                .parse()
                .expect("parsing a constant"),
            ..Self::starknet_mainnet()
        }
    }

    pub fn madara_devnet() -> Self {
        Self {
            chain_name: "Madara".into(),
            chain_id: ChainId::Other("MADARA_DEVNET".into()),
            sequencer_address: Felt::from_hex_unchecked("0x123").try_into().unwrap(),
            ..ChainConfig::starknet_sepolia()
        }
    }

    pub fn madara_test() -> Self {
        Self {
            chain_name: "Test".into(),
            chain_id: ChainId::Other("MADARA_TEST".into()),
            // A random sequencer address for fee transfers to work in block production.
            sequencer_address: Felt::from_hex_unchecked(
                "0x211b748338b39fe8fa353819d457681aa50ac598a3db84cacdd6ece0a17e1f3",
            )
            .try_into()
            .unwrap(),
            ..ChainConfig::starknet_sepolia()
        }
    }

    /// This is the number of pending ticks (see [`ChainConfig::pending_block_update_time`]) in a block.
    pub fn n_pending_ticks_per_block(&self) -> usize {
        (self.block_time.as_millis() / self.pending_block_update_time.as_millis()) as usize
    }

    pub fn exec_constants_by_protocol_version(
        &self,
        version: StarknetVersion,
    ) -> Result<VersionedConstants, UnsupportedProtocolVersion> {
        for (k, constants) in self.versioned_constants.0.iter().rev() {
            if k <= &version {
                return Ok(constants.clone());
            }
        }
        Err(UnsupportedProtocolVersion(version))
    }
}

// TODO: the motivation for these doc comments is to move them into a proper app chain developer documentation, with a
// proper page about tuning the block production performance.
#[derive(Debug, Default)]
pub struct ChainVersionedConstants(pub BTreeMap<StarknetVersion, VersionedConstants>);

impl<const N: usize> From<[(StarknetVersion, VersionedConstants); N]> for ChainVersionedConstants {
    fn from(arr: [(StarknetVersion, VersionedConstants); N]) -> Self {
        ChainVersionedConstants(arr.into_iter().collect())
    }
}

/// Replaces the versioned_constants files definition in the yaml by the content of the
/// jsons.
impl<'de> Deserialize<'de> for ChainVersionedConstants {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let file_paths: BTreeMap<String, String> = Deserialize::deserialize(deserializer)?;
        let mut result = BTreeMap::new();

        for (version, path) in file_paths {
            // Change the current directory to Madara root
            let mut file = File::open(Path::new(&path))
                .with_context(|| format!("Failed to open file: {}", path))
                .map_err(serde::de::Error::custom)?;

            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .with_context(|| format!("Failed to read contents of file: {}", path))
                .map_err(serde::de::Error::custom)?;

            let constants: VersionedConstants = serde_json::from_str(&contents)
                .with_context(|| format!("Failed to parse JSON in file: {}", path))
                .map_err(serde::de::Error::custom)?;

            let parsed_version = version
                .parse()
                .with_context(|| format!("Failed to parse version string: {}", version))
                .map_err(serde::de::Error::custom)?;

            result.insert(parsed_version, constants);
        }

        Ok(ChainVersionedConstants(result))
    }
}

pub fn deserialize_starknet_version<'de, D>(deserializer: D) -> Result<StarknetVersion, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    StarknetVersion::from_str(&s).map_err(serde::de::Error::custom)
}

// TODO: this is workaround because BouncerConfig doesn't derive Deserialize in blockifier
pub fn deserialize_bouncer_config<'de, D>(deserializer: D) -> Result<BouncerConfig, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct BouncerConfigHelper {
        block_max_capacity: BouncerWeights,
    }

    let helper = BouncerConfigHelper::deserialize(deserializer)?;
    Ok(BouncerConfig { block_max_capacity: helper.block_max_capacity })
}

#[cfg(test)]
mod tests {
    use blockifier::{transaction::transaction_types::TransactionType, versioned_constants::ResourceCost};
    use rstest::*;
    use serde_json::Value;
    use starknet_types_core::felt::Felt;

    use super::*;

    #[rstest]
    fn test_mainnet_from_yaml() {
        // Change the current directory
        std::env::set_current_dir("../../../").expect("Failed to change directory");
        let chain_config: ChainConfig =
            ChainConfig::from_yaml(Path::new("configs/presets/mainnet.yaml")).expect("failed to get cfg");

        assert_eq!(chain_config.chain_name, "Starknet Mainnet");
        assert_eq!(chain_config.chain_id, ChainId::Mainnet);

        let native_fee_token_address =
            Felt::from_hex("0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d").unwrap();
        let parent_fee_token_address =
            Felt::from_hex("0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap();
        assert_eq!(chain_config.native_fee_token_address, ContractAddress::try_from(native_fee_token_address).unwrap());
        assert_eq!(chain_config.parent_fee_token_address, ContractAddress::try_from(parent_fee_token_address).unwrap());

        // Check versioned constants
        // Load and parse the JSON file
        let json_content = fs::read_to_string("crates/primitives/chain_config/resources/versioned_constants_13_0.json")
            .expect("Failed to read JSON file");
        let json: Value = serde_json::from_str(&json_content).expect("Failed to parse JSON");

        // Get the VersionedConstants for version 0.13.0
        let constants = chain_config.versioned_constants.0.get(&StarknetVersion::from_str("0.13.0").unwrap()).unwrap();

        // Check top-level fields
        assert_eq!(constants.invoke_tx_max_n_steps, json["invoke_tx_max_n_steps"].as_u64().unwrap() as u32);
        assert_eq!(constants.max_recursion_depth, json["max_recursion_depth"].as_u64().unwrap() as usize);
        assert_eq!(constants.validate_max_n_steps, json["validate_max_n_steps"].as_u64().unwrap() as u32);
        assert_eq!(constants.segment_arena_cells, json["segment_arena_cells"].as_bool().unwrap());

        // Check L2ResourceGasCosts
        let l2_costs = &constants.l2_resource_gas_costs;
        assert_eq!(l2_costs.gas_per_data_felt, ResourceCost::from_integer(0));
        assert_eq!(l2_costs.event_key_factor, ResourceCost::from_integer(0));
        assert_eq!(l2_costs.gas_per_code_byte, ResourceCost::from_integer(0));

        // Check OsConstants
        let os_constants = &constants.os_constants;
        assert_eq!(os_constants.gas_costs.step_gas_cost, json["os_constants"]["step_gas_cost"].as_u64().unwrap());
        assert_eq!(
            os_constants.gas_costs.range_check_gas_cost,
            json["os_constants"]["range_check_gas_cost"].as_u64().unwrap()
        );
        // Add more checks for other gas costs...

        // Check ValidateRoundingConsts
        assert_eq!(os_constants.validate_rounding_consts.validate_block_number_rounding, 1);
        assert_eq!(os_constants.validate_rounding_consts.validate_timestamp_rounding, 1);

        // Check OsResources
        let declare_tx_resources = constants.os_resources_for_tx_type(&TransactionType::Declare, 0);
        assert!(declare_tx_resources.n_steps > 0);

        let invoke_tx_resources = constants.os_resources_for_tx_type(&TransactionType::InvokeFunction, 0);
        assert!(invoke_tx_resources.n_steps > 0);
        // Add more checks for other syscalls and their resources...

        // Check vm_resource_fee_cost using the public method
        let vm_costs = constants.vm_resource_fee_cost();

        // Verify specific resource costs
        assert_eq!(vm_costs.get("n_steps").unwrap(), &ResourceCost::new(5, 1000));
        assert_eq!(vm_costs.get("pedersen_builtin").unwrap(), &ResourceCost::new(16, 100));
        assert_eq!(vm_costs.get("range_check_builtin").unwrap(), &ResourceCost::new(8, 100));
        assert_eq!(vm_costs.get("ecdsa_builtin").unwrap(), &ResourceCost::new(1024, 100));
        assert_eq!(vm_costs.get("bitwise_builtin").unwrap(), &ResourceCost::new(32, 100));
        assert_eq!(vm_costs.get("poseidon_builtin").unwrap(), &ResourceCost::new(16, 100));
        assert_eq!(vm_costs.get("ec_op_builtin").unwrap(), &ResourceCost::new(512, 100));
        assert_eq!(vm_costs.get("keccak_builtin").unwrap(), &ResourceCost::new(1024, 100));

        assert_eq!(chain_config.latest_protocol_version, StarknetVersion::from_str("0.13.2").unwrap());
        assert_eq!(chain_config.block_time, Duration::from_secs(30));
        assert_eq!(chain_config.pending_block_update_time, Duration::from_secs(2));

        // Check bouncer config
        assert_eq!(chain_config.bouncer_config.block_max_capacity.gas, 5000000);
        assert_eq!(chain_config.bouncer_config.block_max_capacity.n_steps, 40000000);
        assert_eq!(chain_config.bouncer_config.block_max_capacity.state_diff_size, 131072);
        assert_eq!(chain_config.bouncer_config.block_max_capacity.builtin_count.add_mod, 18446744073709551615);

        assert_eq!(chain_config.sequencer_address, ContractAddress::try_from(Felt::from_str("0x0").unwrap()).unwrap());
        assert_eq!(chain_config.max_nonce_for_validation_skip, 2);
        assert_eq!(
            chain_config.eth_core_contract_address,
            H160::from_str("0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4").unwrap()
        );
    }

    #[rstest]
    fn test_exec_constants() {
        let chain_config = ChainConfig {
            versioned_constants: [
                (StarknetVersion::new(0, 1, 5, 0), {
                    let mut constants = VersionedConstants::default();
                    constants.validate_max_n_steps = 5;
                    constants
                }),
                (StarknetVersion::new(0, 2, 0, 0), {
                    let mut constants = VersionedConstants::default();
                    constants.validate_max_n_steps = 10;
                    constants
                }),
            ]
            .into(),
            ..ChainConfig::madara_test()
        };

        assert_eq!(
            chain_config
                .exec_constants_by_protocol_version(StarknetVersion::new(0, 1, 5, 0))
                .unwrap()
                .validate_max_n_steps,
            5
        );
        assert_eq!(
            chain_config
                .exec_constants_by_protocol_version(StarknetVersion::new(0, 1, 6, 0))
                .unwrap()
                .validate_max_n_steps,
            5
        );
        assert_eq!(
            chain_config
                .exec_constants_by_protocol_version(StarknetVersion::new(0, 1, 7, 0))
                .unwrap()
                .validate_max_n_steps,
            5
        );
        assert_eq!(
            chain_config
                .exec_constants_by_protocol_version(StarknetVersion::new(0, 2, 0, 0))
                .unwrap()
                .validate_max_n_steps,
            10
        );
        assert_eq!(
            chain_config
                .exec_constants_by_protocol_version(StarknetVersion::new(0, 2, 5, 0))
                .unwrap()
                .validate_max_n_steps,
            10
        );
        assert_eq!(
            chain_config
                .exec_constants_by_protocol_version(StarknetVersion::new(1, 0, 0, 0))
                .unwrap()
                .validate_max_n_steps,
            10
        );
        assert!(chain_config.exec_constants_by_protocol_version(StarknetVersion::new(0, 0, 0, 0)).is_err(),);
    }
}
