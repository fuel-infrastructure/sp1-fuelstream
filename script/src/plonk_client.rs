use anyhow::Result;
use downcast_rs::Downcast;
use primitives::types::ProofInputs;
use sp1_sdk::include_elf;
use sp1_sdk::{
    network::FulfillmentStrategy, CpuProver, EnvProver, HashableKey, NetworkProver, ProverClient,
    SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey,
};
use std::time::Duration;

/// The compiled ELF binary for the FuelStreamX circuit
pub const FUELSTREAMX_ELF: &[u8] = include_elf!("sp1-fuelstreamx-program");

pub struct FuelStreamXPlonkClient {
    prover: EnvProver,
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
        let prover_client = ProverClient::from_env();
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

        // Encode
        let encoded_proof_inputs = serde_cbor::to_vec(&inputs)?;
        stdin.write_vec(encoded_proof_inputs);

        // Generate Proof
        match self.prover.as_any() {
            // If prover is not mocked, it might take a while and requires 128GB+ ram
            prover if prover.is::<CpuProver>() => prover
                .downcast_ref::<CpuProver>()
                .unwrap()
                .prove(&self.pk, &stdin)
                .plonk()
                .run(),
            // Uses Succinct's on-demand prover to fulfill requests
            prover if prover.is::<NetworkProver>() => prover
                .downcast_ref::<NetworkProver>()
                .unwrap()
                .prove(&self.pk, &stdin)
                .strategy(FulfillmentStrategy::Hosted)
                .skip_simulation(true)
                .plonk()
                .timeout(Duration::from_secs(self.timeout))
                .run(),
            _ => panic!("unsupported prover type"),
        }
    }
}
