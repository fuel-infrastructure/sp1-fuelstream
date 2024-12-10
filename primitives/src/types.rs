use alloy::sol;
use serde::{Deserialize, Serialize};
use tendermint::block::Header;
use tendermint_light_client_verifier::types::LightBlock;

/// uint64 trusted_block;
/// bytes32 trusted_header_hash;
/// uint64 target_block;
/// bytes32 target_header_hash;
/// bytes32 bridge_commitment;
pub type ProofOutputs = sol! {
    tuple(uint64, bytes32, uint64, bytes32, bytes32)
};

#[derive(Debug, Serialize, Deserialize)]
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
}
