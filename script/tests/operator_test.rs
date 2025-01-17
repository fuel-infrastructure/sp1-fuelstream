mod common;
#[path = "../bin/operator.rs"]
mod operator;

#[cfg(test)]
mod tests {
    use super::operator::FuelStreamXOperator;

    use crate::common::mock_ethereum_rpc_server::tests::spawn_ethereum_rpc_server;
    use crate::common::mock_tendermint_grpc_server::tests::spawn_tendermint_grpc_server;
    use crate::common::mock_tendermint_rpc_server::tests::spawn_tendermint_rpc_server;
    use crate::common::FULL_RUN;

    use alloy::sol_types::SolType;

    use primitives::types::ProofOutputs;

    use std::env;

    #[test]
    fn operator_run_succeeds() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            // Servers setup
            let eth_rpc_server = spawn_ethereum_rpc_server(FULL_RUN.to_string()).await;
            let tendermint_rpc_server = spawn_tendermint_rpc_server(FULL_RUN.to_string()).await;
            let grpc_url = spawn_tendermint_grpc_server(FULL_RUN.to_string()).await;

            // ================= Ethereum
            env::set_var("RPC_URL", format!("http://{}", eth_rpc_server.address()));
            env::set_var(
                "PRIVATE_KEY",
                "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
            );
            env::set_var(
                "CONTRACT_ADDRESS",
                "0x4e6111c3700cF93E2A3Ac513020e49463A5327b5",
            );

            // ================= Tendermint
            env::set_var(
                "TENDERMINT_RPC_URL",
                format!("http://{}", tendermint_rpc_server.address()),
            );
            env::set_var("TENDERMINT_GRPC_URL", grpc_url);
            env::set_var("TENDERMINT_GRPC_BASIC_AUTH", "anything");

            // ================= SP1
            env::set_var("SP1_PROVER", "mock");

            // ================= General
            env::set_var("MINIMUM_BLOCK_RANGE", "15");

            let mut operator = FuelStreamXOperator::new().await;
            let (proof_outputs, tx_hash) = operator.run().await.unwrap().unwrap();

            assert_eq!(
                "0xd8e75f208091e9bb60a26d8c51aa95cc2af007088cb00a54fba3abe5d6101a4d",
                tx_hash.to_string()
            );

            // Abi-decode the public outputs
            let (
                trusted_height,
                trusted_header_hash,
                target_height,
                target_header_hash,
                bridge_commitment,
            ) = ProofOutputs::abi_decode(&proof_outputs.public_values.to_vec(), true).unwrap();

            // Check that the circuit public outputs are correct.
            // The proof generated is assumed correct and handled by Succinct.
            assert_eq!(177810, trusted_height);
            assert_eq!(
                "0x13416213335b27488b910bbfc1087740f0d3d9844af1b2ffd3eaf75433823ce1",
                trusted_header_hash.to_string()
            );
            assert_eq!(177840, target_height);
            assert_eq!(
                "0x97b040e8c747f83e902ac6f046168c190db913a9eb813e987f1e748656239c3e",
                target_header_hash.to_string()
            );
            assert_eq!(
                "0xe3eeea6996acb801802718794b9611743671e7c33a7a83639e95382ed944f373",
                bridge_commitment.to_string()
            );
        });
    }
}
