use log::debug;
use tendermint_light_client::{
    components::io::{AtHeight, Io},
    state::State,
    types::LightBlock,
    verifier::{
        options::Options,
        types::{Height, Status},
        Verdict, Verifier,
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

    /// Checking for a valid block that the light client can progress to within a given range of blocks.
    /// Since we need to obtain all block headers to reconstruct the bridge commitment within this range,
    /// we must iterate through all the blocks.
    /// Note: an optimization can be done by using a binary search to find the latest valid block first.
    /// After finding the valid block, only the block header rpc is required thus avoiding 1 rpc call
    /// to obtain the validator set per block
    pub async fn skip(&mut self, trusted_height: Height, target_height: Height) {
        // Save the trusted block into state
        let trusted_block = self
            .io
            .fetch_light_block(AtHeight::At(trusted_height))
            .expect("could not 'request' light block");
        self.state
            .light_store
            .insert(trusted_block.clone(), Status::Verified);

        // Loop until we reach the target height, or the first non-trusted block
        // The trusted height is already stored in the state, so skip it
        for current_block in trusted_height.increment().value()..target_height.increment().value() {
            let current_height =
                Height::try_from(current_block).expect("parsed to convert from u64 to Height");

            // Get the untrusted block
            debug!(
                "retrieving block {} from tendermint",
                current_height.value()
            );
            let untrusted_block = self
                .io
                .fetch_light_block(AtHeight::At(current_height))
                .expect("could not 'request' light block");

            // Validate and verify the current block
            let verdict = self.verifier.verify_update_header(
                untrusted_block.as_untrusted_state(),
                trusted_block.as_trusted_state(),
                &self.options,
                untrusted_block.time(),
            );

            // If valid light block, save the block to storage and iterate to the next height
            if verdict == Verdict::Success {
                self.state
                    .light_store
                    .insert(untrusted_block.clone(), Status::Verified);
            } else {
                // If this block was not valid, this indicates that we cannot iterate to the next block
                break;
            }
        }
    }
}
