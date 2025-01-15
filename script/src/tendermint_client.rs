use fuel_sequencer_proto::protos::fuelsequencer::commitments::v1::{
    query_client::QueryClient as CommitmentQueryClient, QueryBridgeCommitmentRequest,
};
use log::debug;
use primitives::get_header_update_verdict;
use primitives::types::ProofInputs;
use std::time::Duration;
use tendermint::block::Header;
use tendermint_light_client::{
    components::io::{AtHeight, Io, ProdIo},
    types::LightBlock,
    verifier::{types::Height, Verdict},
};
use tendermint_rpc::{Client, HttpClient, Url};

use tonic::metadata::MetadataValue;
use tonic::service::interceptor::{InterceptedService, Interceptor};
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::Request;
use tonic::Status;

/// Number of concurrent API requests to a Tendermint node
const BATCH_SIZE: usize = 25;

pub struct FuelStreamXTendermintClient {
    /// A Tendermint RPC client
    pub rpc_client: HttpClient,
    /// Interface for fetching light blocks from a full node
    io: Box<dyn Io>,
    /// The commitment client using the inner `tonic` gRPC channel.
    commitment_client: CommitmentQueryClient<InterceptedService<Channel, AuthInterceptor>>,
}

// Grpc Auth
struct AuthInterceptor {
    auth_token: MetadataValue<tonic::metadata::Ascii>,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        req.metadata_mut()
            .insert("authorization", self.auth_token.clone());
        Ok(req)
    }
}

impl FuelStreamXTendermintClient {
    /// Constructs a new FuelStreamX light client
    pub async fn new(
        tendermint_rpc: Url,
        tendermint_grpc: String,
        auth_basic_grpc: String,
    ) -> Self {
        let rpc_client = HttpClient::new(tendermint_rpc.clone())
            .expect("failed to connect to a tendermint node");

        let peer_id = rpc_client
            .status()
            .await
            .expect("failed to fetch node status")
            .node_info
            .id;

        let timeout = Some(Duration::from_secs(10));
        let io = ProdIo::new(peer_id, rpc_client.clone(), timeout);

        // Grpc
        let channel = Channel::from_shared(tendermint_grpc)
            .expect("failed to parse tendermint grpc endpoint")
            .tls_config(ClientTlsConfig::new())
            .expect("failed to create tls config")
            .connect()
            .await
            .expect("failed to connect with tendermint grpc");

        // Add authorization interceptor
        let auth_token: MetadataValue<_> = format!("Basic {}", auth_basic_grpc).parse().unwrap();
        let interceptor = AuthInterceptor { auth_token };

        let commitment_client = CommitmentQueryClient::with_interceptor(channel, interceptor);

        Self {
            rpc_client,
            io: Box::new(io),
            commitment_client,
        }
    }

    /// Fetches the inputs for the next circuit proof to update the light client on Ethereum.
    pub async fn fetch_proof_inputs(
        &mut self,
        start_height: u64,
        max_end_height: u64,
    ) -> ProofInputs {
        // Check if there was a major voting power change within the given block range
        let (start_light_block, end_light_block) = self
            .get_next_light_client_update(start_height, max_end_height)
            .await;

        // Obtain all the block headers to construct a bridge commitment hash
        let headers = self
            .fetch_blocks_in_range(
                start_light_block.height().value(),
                end_light_block.height().value(),
            )
            .await;

        let bridge_commitment = self
            .fetch_bridge_commitment(
                start_light_block.height().value(),
                end_light_block.height().value(),
            )
            .await;

        ProofInputs {
            trusted_light_block: start_light_block,
            target_light_block: end_light_block,
            headers,
            bridge_commitment,
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
    pub fn fetch_light_block(&mut self, block_height: u64) -> LightBlock {
        debug!("fetching block {} from a Tendermint node", block_height);
        let error_msg = format!("could not request light block {}", block_height);

        self.io
            .fetch_light_block(AtHeight::At(Height::try_from(block_height).unwrap()))
            .expect(&error_msg)
    }

    /// Fetches the bridge commitment between a block range
    pub async fn fetch_bridge_commitment(&mut self, start: u64, end: u64) -> Vec<u8> {
        let req = Request::new(QueryBridgeCommitmentRequest { start, end });

        let resp = self
            .commitment_client
            .bridge_commitment(req)
            .await
            .expect("failed to get a bridge commitment response");

        resp.into_inner().bridge_commitment.to_vec()
    }
}
