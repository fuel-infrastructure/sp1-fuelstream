use log::debug;
use std::time::Duration;
use tendermint::block::Block;
use tendermint_light_client::{
    components::io::{AtHeight, Io, ProdIo},
    state::State,
    types::LightBlock,
    verifier::{
        options::Options,
        types::{Height, Status},
        Verdict,
    },
};
use tendermint_rpc::{Client, HttpClient, Url};

use primitives::get_header_update_verdict;

pub struct FuelStreamXLightClient {
    /// A Tendermint RPC client
    rpc_client: HttpClient,
    /// Interface for fetching light blocks from a full node.
    io: Box<dyn Io>,
}

impl FuelStreamXLightClient {
    /// Constructs a new FuelStreamX light client
    pub async fn new(tendermint_rpc: Url) -> Self {
        let rpc_client =
            HttpClient::new(tendermint_rpc).expect("failed to connect to a tendermint node");

        let peer_id = rpc_client
            .status()
            .await
            .expect("failed to fetch node status")
            .node_info
            .id;

        let timeout = Some(Duration::from_secs(3));
        let io = ProdIo::new(peer_id, rpc_client.clone(), timeout);

        Self {
            rpc_client,
            io: Box::new(io),
        }
    }

    /// Find the next valid block the light client can update to. Binary search is used until a
    /// valid target block is found when max_end_block is not already valid. This occurs when
    /// there was a >33% voting power change and validator signatures from the trusted block
    /// are no longer valid.
    pub async fn get_next_light_client_update(
        &mut self,
        start_block: u64,
        max_end_block: u64,
    ) -> (LightBlock, LightBlock) {
        assert!(start_block < max_end_block, "start_block > max_end_block");
        debug!(
            "finding the next light client header update between blocks {} and {}",
            start_block, max_end_block
        );

        // Trusted block will be used multiple times
        let trusted_block = self.fetch_light_block(start_block);

        // Binary search loop
        let mut curr_end_block = max_end_block;
        while start_block < curr_end_block {
            let untrusted_block = self.fetch_light_block(curr_end_block);

            // Verification
            if Verdict::Success == get_header_update_verdict(&trusted_block, &untrusted_block) {
                return (trusted_block, untrusted_block);
            }

            // If not valid, search in lower half only
            curr_end_block = (start_block + curr_end_block) / 2;
        }

        panic!(
            "could not find any valid untrusted block within the range block {} and {}",
            start_block, max_end_block
        );
    }

    /// Fetches a LightBlock from a CometBFT node. LightBlocks include validator sets.
    fn fetch_light_block(&mut self, block_height: u64) -> LightBlock {
        debug!("fetching block {} from CometBFT", block_height);
        let error_msg = format!("could not request light block {}", block_height);

        self.io
            .fetch_light_block(AtHeight::At(Height::try_from(block_height).unwrap()))
            .expect(&error_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    use core::panic;
    use std::fs;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Fixture contains:
    // Block 177843: Tx submitted to change voting power >66% at
    // Block 177845: Voting power change is committed
    const OVER_66_PERCENT_VOTING_POWER_CHANGE: &str = "over_66%_voting_power_change";

    // The tendermint_light_client library uses synchronous calls, run the tests in async block_on
    // to avoid deadlocks. Don't use tokio's async runtime.
    macro_rules! run_async_test {
        ($test_block:expr) => {{
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on($test_block)
        }};
    }

    // Helper function to load JSON response from filesystem
    fn load_mock_response(fixture_name: &str, filename: &str) -> Value {
        // Load from filesystem
        let content = fs::read_to_string(format!("fixtures/{}/{}", fixture_name, filename))
            .unwrap_or_else(|_| panic!("failed to read mock file: {}", filename));
        // Json Load
        serde_json::from_str(&content).unwrap()
    }

    // Helper function to set up FuelStreamXLightClient pointed to a mocked CometBFT server
    async fn setup_client_with_mocked_server(
        test_name: &'static str,
    ) -> (MockServer, FuelStreamXLightClient) {
        let server = MockServer::start().await;

        // -------- Mock requests

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                let body_str = std::str::from_utf8(&req.body).unwrap();
                let body: serde_json::Value = serde_json::from_str(body_str).unwrap();
                let method = body["method"].as_str().unwrap_or_default();

                match method {
                    "status" => ResponseTemplate::new(200)
                        .set_body_json(load_mock_response(test_name, "status.json")),
                    "commit" => {
                        let height = body["params"]["height"].as_str().unwrap_or("0");
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            test_name,
                            &format!("commit?height={}.json", height),
                        ))
                    }
                    "validators" => {
                        let height = body["params"]["height"].as_str().unwrap_or("0");
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            test_name,
                            &format!("validators?height={}.json", height),
                        ))
                    }
                    _ => panic!("unknown method received, method: {}, {}", method, body_str),
                }
            })
            .mount(&server)
            .await;

        // -------- FuelstreamX setup
        let client =
            FuelStreamXLightClient::new(format!("http://{}", server.address()).parse().unwrap())
                .await;

        (server, client)
    }

    #[test]
    fn next_light_client_update_succeeds_without_binary_search() {
        run_async_test!(async {
            let (_, mut client) =
                setup_client_with_mocked_server(OVER_66_PERCENT_VOTING_POWER_CHANGE).await;
            let (start_block, end_block) =
                client.get_next_light_client_update(177840, 177844).await;

            // No 66% voting power changes, end_block == max_end_block
            assert_eq!(start_block.height().value(), 177840);
            assert_eq!(end_block.height().value(), 177844);
        });
    }

    #[test]
    fn next_light_client_update_succeeds_with_binary_search_loop() {
        run_async_test!(async {
            let (_, mut client) =
                setup_client_with_mocked_server(OVER_66_PERCENT_VOTING_POWER_CHANGE).await;

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

            // Multiple iteration, 177852 -> 177846 -> 177843
            let (start_block, end_block) =
                client.get_next_light_client_update(177840, 177852).await;

            assert_eq!(177840, start_block.height().value());
            assert_eq!(177843, end_block.height().value());
        });
    }

    #[test]
    fn next_light_client_update_succeeds_with_multi_binary_search_loops() {
        // The tendermint_light_client library uses synchronous calls, run the tests in async block_on
        // to avoid deadlocks. Don't use tokio's async runtime.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let (_, mut client) =
                setup_client_with_mocked_server(OVER_66_PERCENT_VOTING_POWER_CHANGE).await;
            let (start_block, end_block) =
                client.get_next_light_client_update(177840, 177850).await;

            // Block 177843: Tx submitted to change voting power >66% at
            // Block 177845: Voting power change is committed
            // Thus; the second mid-point is valid (177850 -> 177845 -> 177842)

            // The mid value for the binary search goes:
            assert_eq!(177840, start_block.height().value());
            assert_eq!(177842, end_block.height().value());
        });
    }

    #[test]
    fn next_light_client_update_succeeds_next_block_is_valid() {
        // The tendermint_light_client library uses synchronous calls, run the tests in async block_on
        // to avoid deadlocks. Don't use tokio's async runtime.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let (_, mut client) =
                setup_client_with_mocked_server(OVER_66_PERCENT_VOTING_POWER_CHANGE).await;

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
        });
    }
}
