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

        let timeout = Some(Duration::from_secs(15));
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
