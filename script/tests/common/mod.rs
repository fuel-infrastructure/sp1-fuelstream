pub mod mock_tendermint_grpc_server;
pub mod mock_tendermint_rpc_server;

use serde_json::Value;
use std::fs;

// Helper function to load JSON response from filesystem
fn load_mock_response(fixture_name: &str, filename: &str) -> Value {
    // Load from filesystem
    let content = fs::read_to_string(format!("fixtures/{}/{}", fixture_name, filename))
        .unwrap_or_else(|_| panic!("failed to read mock file: {}", filename));
    // Json Load
    serde_json::from_str(&content).unwrap()
}
