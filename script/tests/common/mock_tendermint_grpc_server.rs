use super::load_mock_response;

use serde::Deserialize;

use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

use fuel_sequencer_proto::bytes::Bytes;
use fuel_sequencer_proto::protos::fuelsequencer::commitments::v1::{
    query_server::{Query, QueryServer},
    QueryBridgeCommitmentInclusionProofRequest, QueryBridgeCommitmentInclusionProofResponse,
    QueryBridgeCommitmentRequest, QueryBridgeCommitmentResponse,
};

// Needed to load from json
#[derive(Deserialize)]
struct BridgeCommitmentJson {
    bridge_commitment: String,
}

// Server
struct MockCommitmentsService {
    fixture_name: String,
}

#[tonic::async_trait]
impl Query for MockCommitmentsService {
    async fn bridge_commitment(
        &self,
        request: Request<QueryBridgeCommitmentRequest>,
    ) -> Result<Response<QueryBridgeCommitmentResponse>, Status> {
        // Request message
        let inner_request: QueryBridgeCommitmentRequest = request.into_inner();

        // Load from json
        let json_value = load_mock_response(
            &self.fixture_name,
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
                hex::decode(parsed.bridge_commitment).expect("failed to decode bridge commitment"),
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

/// Spawn another thread for the grpc server
pub async fn spawn_grpc_server(fixture_name: &str) -> String {
    // Start gRPC server on a random port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();
    let service = MockCommitmentsService {
        fixture_name: fixture_name.to_string(),
    };

    tokio::spawn(async move {
        Server::builder()
            .add_service(QueryServer::new(service))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("gRPC sequencer server failed")
    });

    format!("http://{}", local_addr)
}
