use std::time::Duration;

use anyhow::Result;
use primitives::types::{ProofInputs, FUELSTREAMX_ELF};
use sp1_sdk::{
    HashableKey, ProverClient, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey,
};

pub struct FuelStreamXPlonkClient {
    prover: ProverClient,
    /// Used to generate a proof for a given RISC-V program.
    pk: SP1ProvingKey,
    /// Used to verify a proof for a given RISC-V program
    vk: SP1VerifyingKey,
    /// Timeout measured in seconds
    pub timeout: u64,
}

impl FuelStreamXPlonkClient {
    /// Constructs a new FuelStreamX plonk client
    pub async fn new(timeout: u64) -> Self {
        let prover_client = ProverClient::new();
        let (pk, vk) = prover_client.setup(FUELSTREAMX_ELF);

        Self {
            prover: prover_client,
            pk,
            vk,
            timeout,
        }
    }

    /// Get the abi-encoded vKey
    pub fn get_v_key_hash(&self) -> String {
        self.vk.bytes32()
    }

    /// Generate a proof using either mock, local or network depending on SP1_PROVER env variable.
    /// Might take some time to compute, adjust SP1_TIMEOUT_MINS accordingly
    pub async fn generate_proof(&self, inputs: ProofInputs) -> Result<SP1ProofWithPublicValues> {
        let mut stdin = SP1Stdin::new();

        // Print headers for debugging
        println!("Headers: {:?}", inputs.headers);

        // Encode
        let encoded_proof_inputs = serde_cbor::to_vec(&inputs)?;
        stdin.write_vec(encoded_proof_inputs);

        // Run, might take a while if on cpu
        self.prover
            .prove(&self.pk, stdin)
            .plonk()
            .timeout(Duration::from_secs(self.timeout))
            .run()
    }
}
