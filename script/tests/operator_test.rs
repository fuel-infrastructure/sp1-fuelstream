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
            operator.run().await;
        });
    }
}
