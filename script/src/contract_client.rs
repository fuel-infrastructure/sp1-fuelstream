use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, B256, U256},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, ProviderBuilder, RootProvider,
    },
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
};
use FuelStreamX::FuelStreamXInstance;

// TODO: link
sol! {
    #[sol(rpc)]
    contract FuelStreamX {
        uint256 public constant BRIDGE_COMMITMENT_MAX;
        uint256 public latestHeight;
        bytes32 public latestBlockHeader;

        function commitHeaderRange(
            bytes calldata proof,
            bytes calldata publicValues
        ) external;
    }
}

/// Alias the fill provider for the Ethereum network. Retrieved from the instantiation of the
/// ProviderBuilder. Recommended method for passing around a ProviderBuilder.
type EthereumFillProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;

pub struct FuelStreamXLightContractClient {
    /// Exposes Ethereum JSON-RPC methods with an Ethereum wallet already configured
    provider: EthereumFillProvider,
    /// FuelStreamX contract instance, connected with the provider
    contract: FuelStreamXInstance<Http<Client>, EthereumFillProvider>,
}

impl FuelStreamXLightContractClient {
    /// Constructs a new FuelStreamX contract client
    pub async fn new(rpc_url: &str, private_key: &str, contract_address: &str) -> Self {
        // The wallet handling the submitting of proofs on Ethereum
        let signer: PrivateKeySigner = private_key.parse().expect("Failed to parse private key");
        let wallet = EthereumWallet::from(signer);

        // Create a provider with the HTTP transport using the `reqwest` crate.
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url.parse().unwrap());

        // Connect with the FuelStreamX contract
        let address = Address::parse_checksummed(contract_address, None)
            .expect("address does not have a valid checksum");
        let contract = FuelStreamX::new(address, provider.clone());

        FuelStreamXLightContractClient { provider, contract }
    }

    /// Get the maximum bridge commitment range allowed
    pub async fn get_bridge_commitment_max(&self) -> U256 {
        self.contract
            .BRIDGE_COMMITMENT_MAX()
            .call()
            .await
            .expect("failed to get BRIDGE_COMMITMENT_MAX")
            .BRIDGE_COMMITMENT_MAX
    }

    /// Get the latest block sync of the light client on Ethereum
    pub async fn get_latest_sync(&self) -> (U256, B256) {
        // Get the latest trusted height
        let latest_height = self
            .contract
            .latestHeight()
            .call()
            .await
            .expect("failed to get latest height from contract")
            .latestHeight;

        // Get the block header hash for the latest trusted height
        let latest_block_header = self
            .contract
            .latestBlockHeader()
            .call()
            .await
            .expect("failed to get latest block header hash from contract")
            .latestBlockHeader;

        (latest_height, latest_block_header)
    }
}
