mod common;

use common::mock_tendermint_grpc_server::spawn_tendermint_grpc_server;
use common::mock_tendermint_rpc_server::spawn_tendermint_rpc_server;
use common::{OVER_66_PERCENT_VOTING_POWER_CHANGE, OVER_85_PERCENT_VOTING_POWER_CHANGE};

use fuelstreamx_sp1_script::tendermint_client::FuelStreamXTendermintClient;

#[cfg(test)]
mod tests {
    use super::*;

    // The tendermint_light_client library uses synchronous calls, run the tests in async block_on
    // to avoid deadlocks. Don't use tokio's async runtime.
    macro_rules! run_async_test {
        ($fixture:expr, $test:expr) => {{
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let grpc_url = spawn_tendermint_grpc_server($fixture.to_string()).await;
                let rpc_url = spawn_tendermint_rpc_server($fixture.to_string()).await;
                let client =
                    FuelStreamXTendermintClient::new(rpc_url.parse().unwrap(), grpc_url).await;

                let test_fn: Box<dyn FnOnce(FuelStreamXTendermintClient) -> _> = Box::new($test);
                test_fn(client).await
            })
        }};
    }

    #[test]
    fn next_light_client_update_succeeds_without_binary_search() {
        run_async_test!(
            OVER_66_PERCENT_VOTING_POWER_CHANGE,
            |mut client| async move {
                let (start_block, end_block) =
                    client.get_next_light_client_update(177840, 177844).await;

                // No 66% voting power changes, end_block == max_end_block
                assert_eq!(start_block.height().value(), 177840);
                assert_eq!(end_block.height().value(), 177844);

                // Odd validator signatures test
                let (start_block, end_block) =
                    client.get_next_light_client_update(177843, 177844).await;

                assert_eq!(177843, start_block.height().value());
                assert_eq!(177844, end_block.height().value());

                // New validator signatures test
                let (start_block, end_block) =
                    client.get_next_light_client_update(177844, 177845).await;

                assert_eq!(177844, start_block.height().value());
                assert_eq!(177845, end_block.height().value());
            }
        );
    }

    #[test]
    fn next_light_client_update_succeeds_with_binary_search_loop() {
        run_async_test!(
            OVER_66_PERCENT_VOTING_POWER_CHANGE,
            |mut client| async move {
                // Single iteration, 177848 -> 177844
                let (start_block, end_block) =
                    client.get_next_light_client_update(177840, 177848).await;

                assert_eq!(177840, start_block.height().value());
                assert_eq!(177844, end_block.height().value());

                // Multiple iteration, 177850 -> 177845 -> 177842
                let (start_block, end_block) =
                    client.get_next_light_client_update(177840, 177850).await;

                assert_eq!(177840, start_block.height().value());
                assert_eq!(177842, end_block.height().value());
            }
        );
    }

    #[test]
    fn next_light_client_update_succeeds_without_binary_search_over_85() {
        run_async_test!(
            OVER_85_PERCENT_VOTING_POWER_CHANGE,
            |mut client| async move {
                let (start_block, end_block) =
                    client.get_next_light_client_update(215198, 215201).await;

                // No 33% voting power changes, end_block == max_end_block
                assert_eq!(start_block.height().value(), 215198);
                assert_eq!(end_block.height().value(), 215201);

                // Odd validator signatures test
                let (start_block, end_block) =
                    client.get_next_light_client_update(215200, 215201).await;

                assert_eq!(215200, start_block.height().value());
                assert_eq!(215201, end_block.height().value());

                // New validator signatures test
                let (start_block, end_block) =
                    client.get_next_light_client_update(215201, 215202).await;

                assert_eq!(215201, start_block.height().value());
                assert_eq!(215202, end_block.height().value());
            }
        );
    }

    #[test]
    fn next_light_client_update_succeeds_with_binary_search_loop_over_85() {
        run_async_test!(
            OVER_85_PERCENT_VOTING_POWER_CHANGE,
            |mut client| async move {
                // Single iteration, 215198 -> 215201
                let (start_block, end_block) =
                    client.get_next_light_client_update(215198, 215205).await;

                assert_eq!(215198, start_block.height().value());
                assert_eq!(215201, end_block.height().value());

                // Multiple iteration, 215200 -> 215202 -> 215201
                let (start_block, end_block) =
                    client.get_next_light_client_update(215200, 215205).await;

                assert_eq!(215200, start_block.height().value());
                assert_eq!(215201, end_block.height().value());
            }
        );
    }

    #[test]
    fn fetch_blocks_in_range_succeeds() {
        run_async_test!(
            OVER_85_PERCENT_VOTING_POWER_CHANGE,
            |mut client| async move {
                let headers = client.fetch_blocks_in_range(215198, 215207).await;

                // We only care about last_results_hash from the headers
                assert_eq!(
                    headers
                        .iter()
                        .map(|h| h.last_results_hash.unwrap().to_string())
                        .collect::<Vec<_>>(),
                    vec![
                        "A764280DBA00197147BF3204DA21066B6EE8C79100890D610533F3471B645B01",
                        "CEB59E62AC65B4E1F13BEB5507A7F94F7CAE1E282987A4FD92D5F95B70BEFAB7",
                        "CEB59E62AC65B4E1F13BEB5507A7F94F7CAE1E282987A4FD92D5F95B70BEFAB7",
                        "7F66114C5E082937B2CE4E4828C0CBD915C08EE0E4191A35700C88AF67BAC7DB",
                        "D7224E4D3DA68255E2F44060EF752B8CFCECA9B07D3BE48D569ADD51624F9F97",
                        "7A470E7513D8CA3D2B8E84FA8A009F85C7F92B902B1D1BB57BDBF886F00F65DB",
                        "CB8F355EE7ECBD4963321EDDE78F919E6B16745F5BB64DCE1AA8E3983B5B3A00",
                        "AD8A7992BAE27F7AFF45FA131212003B1965101FD7273BF86010E1EEC86C7407",
                        "776BAC2BEA051A2B6F1CCDA4F0832DC44C8D1A9103B2AEFA6DFE054101FC4CDD"
                    ]
                );
            }
        );
    }

    #[test]
    fn fetch_bridge_commitment_succeeds() {
        run_async_test!(
            OVER_85_PERCENT_VOTING_POWER_CHANGE,
            |mut client| async move {
                // Get commitment within a range
                let bridge_commitment = client.fetch_bridge_commitment(215198, 215207).await;

                assert_eq!(
                    hex::encode(bridge_commitment).to_uppercase(),
                    "3E65AF4686FAE0D1E20903DA42F94A06E74034D5220FA14A5C33A24B928558A8",
                );
            }
        );
    }
}
