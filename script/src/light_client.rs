use log::debug;
use tendermint_light_client::{
    components::io::{AtHeight, Io},
    state::State,
    types::LightBlock,
    verifier::{
        options::Options,
        types::{Height, Status},
        Verdict,
    },
};

use primitives::get_header_update_verdict;

pub struct FuelStreamXLightClient {
    /// The default tendermint light client storage
    pub state: State,
    /// Handles the connection with a tendermint chain
    io: Box<dyn Io>,
    /// Contains the tendermint chain options, including the trusting period and the trust threshold
    options: Options,
}

impl FuelStreamXLightClient {
    /// Constructs a new FuelStreamX light client
    pub fn new(io: impl Io + 'static, state: State, options: Options) -> Self {
        Self {
            io: Box::new(io),
            state,
            options,
        }
    }

    /// Fetches a light block from a CometBFT node and stores it in the local state.
    fn fetch_and_store_light_block(&mut self, block_height: u64, status: Status) -> LightBlock {
        debug!("fetching block {} from CometBFT", block_height);

        let block = self
            .io
            .fetch_light_block(AtHeight::At(Height::try_from(block_height).unwrap()))
            .expect(&format!("could not request light block {}", block_height));

        self.state.light_store.insert(block.clone(), status);

        return block;
    }

    /// Find the next valid block the light client can iterate to. Binary search is used if
    /// max_end_block is not already valid. All fetched LightBlocks are stored in the local state.
    pub async fn get_next_block_sync(&mut self, start_block: u64, max_end_block: u64) -> u64 {
        assert!(start_block < max_end_block, "start_block > max_end_block");
        debug!(
            "finding the next light client header update between blocks {} and {}",
            start_block, max_end_block
        );

        // Store the blocks for future use
        let trusted_block = self.fetch_and_store_light_block(start_block, Status::Verified);
        let untrusted_block = self.fetch_and_store_light_block(max_end_block, Status::Unverified);

        // If max_end_block is already valid, return it
        if Verdict::Success == get_header_update_verdict(&trusted_block, &untrusted_block) {
            self.state
                .light_store
                .insert(untrusted_block, Status::Verified);

            return max_end_block;
        }

        // Else, find the first untrusted block using binary search
        let mut left = start_block;
        let mut right = max_end_block;
        let mut last_trusted = left;
        while left + 1 < right {
            let mid = left + (right - left) / 2;
            let untrusted_block = self.fetch_and_store_light_block(mid, Status::Unverified);

            // Verification step
            match get_header_update_verdict(&trusted_block, &untrusted_block) {
                // If mid block is trusted, search in upper half
                Verdict::Success => {
                    last_trusted = mid;
                    left = mid;

                    // LightBlock is now trust-worthy
                    self.state
                        .light_store
                        .insert(untrusted_block.clone(), Status::Verified);
                }
                // If mid block is not trusted, search in lower half
                _ => {
                    right = mid;

                    self.state
                        .light_store
                        .insert(untrusted_block.clone(), Status::Unverified);
                }
            }
        }

        return last_trusted;
    }
}
