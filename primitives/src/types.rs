use alloy::sol;
use serde::{Deserialize, Serialize};
use tendermint::block::Header;
use tendermint_light_client_verifier::types::LightBlock;

/// The compiled ELF binary for the FuelStreamX circuit
pub const FUELSTREAMX_ELF: &[u8] = include_bytes!("../../elf/fuelstreamx-elf");

/// Follows the structure as defined in:
/// https://github.com/fuel-infrastructure/fuel-sequencer/blob/538bcdb449ba86f3db6d774c37d99056aa877f80/proto/fuelsequencer/commitments/types.proto#L9
pub type BridgeCommitmentLeaf = sol! {
    tuple(uint64, bytes32)
};

/// Follows the structure as defined in:
/// TODO: link
pub type ProofOutputs = sol! {
    tuple(uint64, bytes32, uint64, bytes32, bytes32)
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofInputs {
    /// The trusted light block containing validator sets required for light client verification.
    /// This block serves as the starting point for the light client's verification process.
    pub trusted_light_block: LightBlock,

    /// The target light block to be verified against the trusted light block.
    /// This block represents the endpoint of the verification process.
    pub target_light_block: LightBlock,

    /// Intermediate headers required to reconstruct the bridge commitment.
    /// This vector contains all headers between (but not including) the trusted
    /// and target light blocks' headers.
    pub headers: Vec<Header>,

    /// The bridge commitment for the range of the block.
    /// In the circuit we re-construct the bridge commitment. We pass the commitment here
    /// to re-assure everything is correct code-wise.
    pub bridge_commitment: Vec<u8>,
}
