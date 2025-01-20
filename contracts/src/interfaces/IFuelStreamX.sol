// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface IFuelStreamX {
    /// @notice Emits event with the new head update.
    event HeadUpdate(uint64 blockNumber, bytes32 headerHash);

    /// @notice Trusted header not found.
    error TrustedHeaderNotFound();

    /// @notice Target block for proof must be greater than latest block and less than the
    /// latest block plus the maximum number of skipped blocks.
    error TargetBlockNotInRange();

    /// @notice Trusted header mismatch.
    error TrustedHeaderMismatch();

    /// @notice Data commitment stored for the block range [startBlock, endBlock) with proof nonce.
    /// @param proofNonce The nonce of the proof.
    /// @param startBlock The start block of the block range.
    /// @param endBlock The end block of the block range.
    /// @param dataCommitment The data commitment for the block range.
    event BridgeCommitmentStored(
        uint256 proofNonce,
        uint64 indexed startBlock,
        uint64 indexed endBlock,
        bytes32 indexed dataCommitment
    );
}
