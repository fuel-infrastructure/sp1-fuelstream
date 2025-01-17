mod common;

#[cfg(test)]
mod tests {
    use crate::common::mock_ethereum_rpc_server::tests::spawn_ethereum_rpc_server;
    use crate::common::OVER_66_PERCENT_VOTING_POWER_CHANGE;

    use fuelstreamx_sp1_script::ethereum_client::FuelStreamXEthereumClient;

    macro_rules! run_async_test {
        ($fixture:expr, $test:expr) => {{
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let eth_rpc_server =
                    spawn_ethereum_rpc_server(OVER_66_PERCENT_VOTING_POWER_CHANGE.to_string())
                        .await;
                let client = FuelStreamXEthereumClient::new(
                    &format!("http://{}", eth_rpc_server.address()),
                    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
                    "0x5FbDB2315678afecb367f032d93F642f64180aa3",
                )
                .await;

                let test_fn: Box<dyn FnOnce(FuelStreamXEthereumClient) -> _> = Box::new($test);
                test_fn(client).await
            })
        }};
    }

    #[test]
    fn fetch_contract_data_success() {
        run_async_test!(OVER_66_PERCENT_VOTING_POWER_CHANGE, |client| async move {
            // Commitment
            let bridge_commitment_max = client.get_bridge_commitment_max().await;

            assert_eq!(4096, bridge_commitment_max);

            // Genesis values
            let (latest_height, latest_block_header) = client.get_latest_sync().await;

            assert_eq!(1, latest_height);
            assert_eq!(
                "0xd024b653e1eaecfb8ed7b87ee5123892b5f14a00dade2f6c41ece68e9e9d2b82",
                latest_block_header.to_string()
            );

            // vKey
            let v_key = client.get_v_key().await;

            assert_eq!(
                "0x005ea3520b9277a62c636dabcf2e7930aed0b99766d8d741f179261aecdd9e71",
                v_key.to_string()
            );
        });
    }
}
