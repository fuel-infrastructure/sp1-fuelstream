use log::debug;
use std::time::Duration;
use tendermint::block::Header;
use tendermint_light_client::{
    components::io::{AtHeight, Io, ProdIo},
    types::LightBlock,
    verifier::{types::Height, Verdict},
};
use tendermint_rpc::{Client, HttpClient, Url};

use primitives::get_header_update_verdict;

/// Number of concurrent API requests to a Tendermint node
const BATCH_SIZE: usize = 25;

pub struct FuelStreamXLightClient {
    /// A Tendermint RPC client
    rpc_client: HttpClient,
    /// Interface for fetching light blocks from a full node
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

    /// Find the next valid block the light client can update to. Lower binary search is used until
    /// a valid target block is found when max_end_block is not already valid. This occurs when
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
                debug!(
                    "next light client header update between blocks {} and {}",
                    trusted_block.height().value(),
                    untrusted_block.height().value()
                );
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

    /// Get a block header within a range, end exclusive. Does not obtain the validators' voting
    /// power.
    pub async fn fetch_blocks_in_range(&self, start_block: u64, end_block: u64) -> Vec<Header> {
        assert!(start_block < end_block, "start_block > max_end_block");
        debug!(
            "fetching light blocks between blocks {} and {}",
            start_block, end_block,
        );

        let mut blocks = Vec::new();

        for batch_start in (start_block..end_block).step_by(BATCH_SIZE) {
            let mut batch_futures = Vec::with_capacity(BATCH_SIZE);

            // Get block commits concurrently, end exclusive
            for height in
                batch_start..std::cmp::min(batch_start + (BATCH_SIZE as u64) - 1, end_block)
            {
                batch_futures.push(async move {
                    self.rpc_client
                        .commit(Height::try_from(height).unwrap())
                        .await
                });
            }

            // Wait for all futures in the batch to complete
            let batch_blocks = futures::future::join_all(batch_futures).await;
            blocks.extend(
                batch_blocks
                    .into_iter()
                    .map(|r| r.expect("failed to fetch block").signed_header.header),
            );
        }

        debug!(
            "finished fetching light blocks between blocks {} and {}",
            start_block, end_block,
        );

        blocks
    }

    /// Fetches a LightBlock from a Tendermint node. LightBlocks include validator sets.
    fn fetch_light_block(&mut self, block_height: u64) -> LightBlock {
        debug!("fetching block {} from a Tendermint node", block_height);
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

    // Fixtures contains:
    // Block 177843: Tx submitted to change voting power >66% at
    // Block 177845: Voting power change is committed
    const OVER_66_PERCENT_VOTING_POWER_CHANGE: &str = "over_66%_voting_power_change";

    // Fixtures contains:
    // Block 215200: Tx submitted to change voting power >80% at
    // Block 215202: Voting power change is committed
    const OVER_85_PERCENT_VOTING_POWER_CHANGE: &str = "over_85%_voting_power_change";

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

    // Helper function to set up FuelStreamXLightClient pointed to a mocked Tendermint node
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
        });
    }

    #[test]
    fn next_light_client_update_succeeds_without_binary_search_over_85() {
        run_async_test!(async {
            let (_, mut client) =
                setup_client_with_mocked_server(OVER_85_PERCENT_VOTING_POWER_CHANGE).await;
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
        });
    }

    #[test]
    fn next_light_client_update_succeeds_with_binary_search_loop_over_85() {
        run_async_test!(async {
            let (_, mut client) =
                setup_client_with_mocked_server(OVER_85_PERCENT_VOTING_POWER_CHANGE).await;

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
        });
    }

    #[test]

    fn fetch_blocks_in_range_succeeds() {
        run_async_test!(async {
            let (_, client) =
                setup_client_with_mocked_server(OVER_85_PERCENT_VOTING_POWER_CHANGE).await;

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
        });
    }
}
