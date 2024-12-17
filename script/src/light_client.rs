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

    /// Find the next valid block the light client can iterate to. Binary search is used if
    /// max_end_block is not already valid.
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

        // Store the blocks for future use
        let trusted_block = self.fetch_light_block(start_block);
        let untrusted_block = self.fetch_light_block(max_end_block);

        // If max_end_block height is already valid, return it
        if Verdict::Success == get_header_update_verdict(&trusted_block, &untrusted_block) {
            return (trusted_block, untrusted_block);
        }

        // Else, find the first untrusted block using binary search
        let mut left = start_block;
        let mut right = max_end_block;
        let mut last_trusted = left;
        while left + 1 < right {
            let mid = left + (right - left) / 2;
            let untrusted_block = self.fetch_light_block(mid);

            // Verification step
            match get_header_update_verdict(&trusted_block, &untrusted_block) {
                // If mid block is trusted, search in upper half
                Verdict::Success => {
                    last_trusted = mid;
                    left = mid;
                }
                // If mid block is not trusted, search in lower half
                _ => {
                    right = mid;
                }
            }
        }

        // TODO: test this function
        return (trusted_block, untrusted_block);
    }

    /// Fetches a LightBlock from a CometBFT node. LightBlocks include validator sets.
    fn fetch_light_block(&mut self, block_height: u64) -> LightBlock {
        debug!("fetching block {} from CometBFT", block_height);

        let block = self
            .io
            .fetch_light_block(AtHeight::At(Height::try_from(block_height).unwrap()))
            .expect(&format!("could not request light block {}", block_height));

        return block;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    use core::panic;
    use std::fs;
    use std::str::from_utf8;

    use wiremock::matchers::{any, method, path, query_param};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    // Helper function to load JSON response from file
    fn load_mock_response(filename: &str) -> Value {
        let content = fs::read_to_string(format!("fixtures/{}", filename))
            .unwrap_or_else(|_| panic!("failed to read mock file: {}", filename));
        serde_json::from_str(&content).unwrap()
    }

    #[test]
    fn test_light_client_with_mock_responses() {
        // Create a new runtime for this test
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Run the async test code in blocking mode
        runtime.block_on(async {
            let server = MockServer::start().await;

            // -------- Mock requests

            // All CometBFT requests are POST, the method called is found inside the request's body
            Mock::given(method("POST"))
                .and(path("/"))
                .respond_with(|req: &wiremock::Request| {
                    let body_str = std::str::from_utf8(&req.body).unwrap();
                    let body: serde_json::Value = serde_json::from_str(body_str).unwrap();

                    // Extract the method from the request body
                    let method = body["method"].as_str().unwrap_or_default();

                    match method {
                        "status" => ResponseTemplate::new(200)
                            .set_body_json(load_mock_response("status.json")),
                        "commit" => {
                            // Extract height from params
                            let height = body["params"]["height"].as_str().unwrap_or("0");

                            ResponseTemplate::new(200).set_body_json(load_mock_response(&format!(
                                "commit?height={}.json",
                                height
                            )))
                        }
                        "validators" => {
                            // Extract height from params
                            let height = body["params"]["height"].as_str().unwrap_or("0");

                            ResponseTemplate::new(200).set_body_json(load_mock_response(&format!(
                                "validators?height={}.json",
                                height
                            )))
                        }
                        _ => {
                            panic!("unknown method received, method: {}, {}", method, body_str);
                        }
                    }
                })
                .mount(&server)
                .await;

            // Setup
            let server_url = format!("http://{}", server.address()).parse().unwrap();
            println!("{}", &server_url);
            let mut client = FuelStreamXLightClient::new(server_url).await;
            let (start_block, end_block) =
                client.get_next_light_client_update(177840, 177850).await;
        });
    }
}
