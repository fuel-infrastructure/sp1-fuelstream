use core::panic;
use fuel_sequencer_proto::bytes::Bytes;
use fuel_sequencer_proto::protos::fuelsequencer::commitments::v1::{
    query_server::{Query, QueryServer},
    QueryBridgeCommitmentInclusionProofRequest, QueryBridgeCommitmentInclusionProofResponse,
    QueryBridgeCommitmentResponse,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

mod common;
use common::MockTendermintGrpcServer;
use fuelstreamx_sp1_script::tendermint_client::FuelStreamXTendermintClient;

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

// Helper function to set up FuelStreamXTendermintClient pointed to a mocked Tendermint node
async fn setup_client_with_mocked_server(
    test_name: &'static str,
) -> (MockServer, FuelStreamXTendermintClient) {
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

    // -------- Mock gRPC server
    struct MockQueryService {
        test_name: &'static str,
    }

    #[derive(Deserialize)]
    struct BridgeCommitmentJson {
        bridge_commitment: String,
    }

    #[tonic::async_trait]
    impl Query for MockQueryService {
        async fn bridge_commitment(
            &self,
            request: Request<QueryBridgeCommitmentRequest>,
        ) -> Result<Response<QueryBridgeCommitmentResponse>, Status> {
            // Request message
            let inner_request: QueryBridgeCommitmentRequest = request.into_inner();

            // Load from json
            let json_value = load_mock_response(
                self.test_name,
                &format!(
                    "bridge_commitment?start={}&end={}.json",
                    inner_request.start, inner_request.end
                ),
            );
            // Parse
            let parsed: BridgeCommitmentJson =
                serde_json::from_value(json_value).expect("failed to deserialized json");

            // Create response
            let response = QueryBridgeCommitmentResponse {
                bridge_commitment: Bytes::from(
                    hex::decode(parsed.bridge_commitment)
                        .expect("failed to decode bridge commitment"),
                ),
            };
            Ok(Response::new(response))
        }
        // All other methods return unimplemented
        async fn bridge_commitment_inclusion_proof(
            &self,
            _request: Request<QueryBridgeCommitmentInclusionProofRequest>,
        ) -> Result<Response<QueryBridgeCommitmentInclusionProofResponse>, Status> {
            Err(Status::unimplemented("method not implemented"))
        }
    }

    // Start gRPC server on a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let service = MockQueryService { test_name };

    tokio::spawn(async move {
        Server::builder()
            .add_service(QueryServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("gRPC sequencer server failed")
    });

    // -------- FuelstreamX setup
    let client = FuelStreamXTendermintClient::new(
        format!("http://{}", server.address()).parse().unwrap(),
        format!("http://{}", local_addr),
    )
    .await;

    (server, client)
}

#[test]
fn next_light_client_update_succeeds_without_binary_search() {
    run_async_test!(async {
        let (_, mut client) =
            setup_client_with_mocked_server(OVER_66_PERCENT_VOTING_POWER_CHANGE).await;
        let (start_block, end_block) = client.get_next_light_client_update(177840, 177844).await;

        // No 66% voting power changes, end_block == max_end_block
        assert_eq!(start_block.height().value(), 177840);
        assert_eq!(end_block.height().value(), 177844);

        // Odd validator signatures test
        let (start_block, end_block) = client.get_next_light_client_update(177843, 177844).await;

        assert_eq!(177843, start_block.height().value());
        assert_eq!(177844, end_block.height().value());

        // New validator signatures test
        let (start_block, end_block) = client.get_next_light_client_update(177844, 177845).await;

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
        let (start_block, end_block) = client.get_next_light_client_update(177840, 177848).await;

        assert_eq!(177840, start_block.height().value());
        assert_eq!(177844, end_block.height().value());

        // Multiple iteration, 177850 -> 177845 -> 177842
        let (start_block, end_block) = client.get_next_light_client_update(177840, 177850).await;

        assert_eq!(177840, start_block.height().value());
        assert_eq!(177842, end_block.height().value());
    });
}

#[test]
fn next_light_client_update_succeeds_without_binary_search_over_85() {
    run_async_test!(async {
        let (_, mut client) =
            setup_client_with_mocked_server(OVER_85_PERCENT_VOTING_POWER_CHANGE).await;
        let (start_block, end_block) = client.get_next_light_client_update(215198, 215201).await;

        // No 33% voting power changes, end_block == max_end_block
        assert_eq!(start_block.height().value(), 215198);
        assert_eq!(end_block.height().value(), 215201);

        // Odd validator signatures test
        let (start_block, end_block) = client.get_next_light_client_update(215200, 215201).await;

        assert_eq!(215200, start_block.height().value());
        assert_eq!(215201, end_block.height().value());

        // New validator signatures test
        let (start_block, end_block) = client.get_next_light_client_update(215201, 215202).await;

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
        let (start_block, end_block) = client.get_next_light_client_update(215198, 215205).await;

        assert_eq!(215198, start_block.height().value());
        assert_eq!(215201, end_block.height().value());

        // Multiple iteration, 215200 -> 215202 -> 215201
        let (start_block, end_block) = client.get_next_light_client_update(215200, 215205).await;

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

#[test]
fn fetch_bridge_commitment_succeeds() {
    run_async_test!(async {
        let (_, mut client) =
            setup_client_with_mocked_server(OVER_85_PERCENT_VOTING_POWER_CHANGE).await;

        // Get commitment within a range
        let bridge_commitment = client.fetch_bridge_commitment(215198, 215207).await;

        assert_eq!(
            hex::encode(bridge_commitment).to_uppercase(),
            "3E65AF4686FAE0D1E20903DA42F94A06E74034D5220FA14A5C33A24B928558A8",
        );
    });
}
