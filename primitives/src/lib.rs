use std::time::Duration;

use tendermint_light_client_verifier::{
    options::Options,
    types::{LightBlock, TrustThreshold},
    ProdVerifier, Verdict, Verifier,
};

pub mod types;

/// Get the verdict for the header update from trusted_block to target_block.
pub fn get_header_update_verdict(trusted_block: &LightBlock, target_block: &LightBlock) -> Verdict {
    let opt = Options {
        // Note: For additional security, set the trust threshold to 2/3.
        trust_threshold: TrustThreshold::TWO_THIRDS,
        // 10 days trusting period is valid for chains with 14 day unbonding period.
        trusting_period: Duration::from_secs(10 * 24 * 60 * 60),
        clock_drift: Duration::ZERO,
    };

    let vp = ProdVerifier::default();

    // Note: The zkVM has no notion of time, so no header will be rejected for being too
    // far in the past, which is a potential issue. Deployers must ensure that the target block is not
    // too far in the past, i.e. the light client must be relatively synced with the chain (i.e.
    // within the trusting period).
    // TODO: https://github.com/fuel-infrastructure/sp1-fuelstream/issues/1
    let verify_time = target_block.time() + Duration::from_secs(10);
    vp.verify_update_header(
        target_block.as_untrusted_state(),
        trusted_block.as_trusted_state(),
        &opt,
        verify_time.unwrap(),
    )
}
