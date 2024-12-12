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

pub struct FuelStreamXLightClient {
    /// The default tendermint light client storage
    pub state: State,
    /// Handles the connection with a tendermint chain
    io: Box<dyn Io>,
    /// The default tendermint light client verifier
    verifier: Box<dyn Verifier>,
    /// Contains the tendermint chain options, including the trusting period and the trust threshold
    options: Options,
}

impl FuelStreamXLightClient {
    /// Constructs a new FuelStreamX light client
    pub fn new(
        io: impl Io + 'static,
        verifier: impl Verifier + 'static,
        state: State,
        options: Options,
    ) -> Self {
        Self {
            io: Box::new(io),
            verifier: Box::new(verifier),
            state,
            options,
        }
    }

    /// Fetches a light block from a CometBFT node and stores it in the local state.
    fn fetch_and_store_light_block(&mut self, block_height: u64, store: bool) -> LightBlock {
        debug!("fetching block {} from CometBFT", block_height);

        let block = self
            .io
            .fetch_light_block(AtHeight::At(Height::try_from(block_height).unwrap()))
            .expect(&format!("could not request light block {}", block_height));

        if store {
            self.state
                .light_store
                .insert(block.clone(), Status::Verified);
        }

        return block;
    }

    pub async fn get_next_block_sync(&mut self, start_block: u64, max_end_block: u64) -> u64 {
        debug!(
            "finding next light client step between blocks {} and {}",
            start_block, max_end_block
        );

        // Binary search to find the end block height
        let trusted_block = self.fetch_and_store_light_block(start_block, true);
        let mut curr_end_block = max_end_block;
        loop {
            // TODO: does this makes sense? it needs always + 1
            if curr_end_block - start_block == 1 {
                return curr_end_block;
            }

            // Don't storing since it's possible the light block is not valid
            let untrusted_block = self.fetch_and_store_light_block(curr_end_block, false);

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
                return curr_end_block;
            }

            // If this block was not valid, this indicates that we cannot iterate to the next block
            let mid_block = (curr_end_block + start_block) / 2;
            curr_end_block = mid_block;
        }
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
