pub mod mock_ethereum_rpc_server;
pub mod mock_tendermint_grpc_server;
pub mod mock_tendermint_rpc_server;

// Fixture contains:
// Block 177843: Tx submitted to change voting power >66% at
// Block 177845: Voting power change is committed
#[cfg(test)]
pub const OVER_66_PERCENT_VOTING_POWER_CHANGE: &str = "over_66%_voting_power_change";

// Fixture contains:
// Block 215200: Tx submitted to change voting power >80% at
// Block 215202: Voting power change is committed
#[cfg(test)]
pub const OVER_85_PERCENT_VOTING_POWER_CHANGE: &str = "over_85%_voting_power_change";

// Fixture contains:
// All blocks from 1 - 30, with bridge commitment max being 30.
#[cfg(test)]
pub const FULL_RUN: &str = "full_run";

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use std::fs;

    // Helper function to load JSON response from filesystem
    #[cfg(test)]
    pub fn load_mock_response(fixture_name: &str, filename: &str) -> Value {
        // Load from filesystem
        let content = fs::read_to_string(format!("tests/fixtures/{}/{}", fixture_name, filename))
            .unwrap_or_else(|_| panic!("failed to read mock file: {}", filename));
        // Json Load
        serde_json::from_str(&content).unwrap()
    }
}
