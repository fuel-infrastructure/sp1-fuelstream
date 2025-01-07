#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::primitives::B256;
use alloy::sol_types::SolType;
use primitives::get_header_update_verdict;
use primitives::types::{BridgeCommitmentLeaf, ProofInputs, ProofOutputs};
use sha2::Sha256;
use tendermint::{block::Header, merkle::simple_hash_from_byte_vectors};
use tendermint_light_client_verifier::Verdict;

/// Compute the bridge commitment for the supplied headers. Each leaf in the Tendermint Merkle tree
/// is the SHA256 hash of the concatenation of the block height and the header's last
/// `LastResultsHash`. Excludes the last header's last results hash from the commitment to avoid
/// overlapping headers between commits.
fn compute_bridge_commitment(headers: &[Header]) -> [u8; 32] {
    let mut encoded_leaves: Vec<Vec<u8>> = Vec::new();
    // Loop over all headers except the last one.
    for i in 0..headers.len() - 1 {
        let curr_header = &headers[i];
        let next_header = &headers[i + 1];

        // Verify the chain of headers is connected.
        if curr_header.hash() != next_header.last_block_id.unwrap().hash {
            panic!("invalid header");
        }

        let last_results_hash: [u8; 32] = curr_header
            .last_results_hash
            .expect("header has no last results hash.")
            .as_bytes()
            .try_into()
            .unwrap();

        // ABI-encode the leaf corresponding to this header, which is a BridgeCommitmentLeaf.
        let encoded_leaf =
            BridgeCommitmentLeaf::abi_encode(&(&curr_header.height.value(), last_results_hash));
        encoded_leaves.push(encoded_leaf);
    }

    // Return the root of the Tendermint Merkle tree.
    simple_hash_from_byte_vectors::<Sha256>(&encoded_leaves)
}

pub fn main() {
    // Read in the proof inputs. Note: Use a slice, as bincode is unable to deserialize protobuf.
    let proof_inputs_vec = sp1_zkvm::io::read_vec();
    let proof_inputs = serde_cbor::from_slice(&proof_inputs_vec).unwrap();

    let ProofInputs {
        trusted_light_block,
        target_light_block,
        headers,
    } = proof_inputs;

    let verdict = get_header_update_verdict(&trusted_light_block, &target_light_block);

    // If the Verdict is not Success, panic.
    match verdict {
        Verdict::Success => (),
        Verdict::NotEnoughTrust(voting_power_tally) => {
            panic!(
                "not enough trust in the trusted header, voting power tally: {:?}",
                voting_power_tally
            );
        }
        Verdict::Invalid(err) => panic!(
            "could not verify updating to target_block, error: {:?}",
            err
        ),
    }

    // Compute the bridge commitment across the range.
    let mut all_headers = Vec::new();
    all_headers.push(trusted_light_block.signed_header.header.clone());
    all_headers.extend(headers);
    all_headers.push(target_light_block.signed_header.header.clone());
    let bridge_commitment = B256::from_slice(&compute_bridge_commitment(&all_headers));

    // ABI encode the proof outputs to bytes and commit them to the zkVM.
    let trusted_header_hash =
        B256::from_slice(trusted_light_block.signed_header.header.hash().as_bytes());
    let target_header_hash =
        B256::from_slice(target_light_block.signed_header.header.hash().as_bytes());
    let proof_outputs = ProofOutputs::abi_encode(&(
        trusted_light_block.signed_header.header.height.value(),
        trusted_header_hash,
        target_light_block.signed_header.header.height.value(),
        target_header_hash,
        bridge_commitment,
    ));
    sp1_zkvm::io::commit_slice(&proof_outputs);
}
