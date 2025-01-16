// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IFuelStreamX} from "./interfaces/IFuelStreamX.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

contract FuelStreamX is IFuelStreamX {
    /// @notice The block is the first one in the next data commitment.
    uint64 public latestBlock;

    /// @notice The maximum number of blocks that can be skipped in a single request. Should be
    /// large enough to skip forward at least 4 hours.
    uint64 public constant BRIDGE_COMMITMENT_MAX = 4096;

    /// @notice Nonce for proof events. Must be incremented sequentially.
    uint256 public state_proofNonce;

    /// @notice Maps block heights to their header hashes.
    mapping(uint64 => bytes32) public blockHeightToHeaderHash;

    /// @notice Mapping of data commitment nonces to data commitments.
    mapping(uint256 => bytes32) public state_bridgeCommitments;

    /// @notice Time after which a commit becomes finalized
    uint32 public timeToFinalize;

    /// @notice The verification key for the ZK circuit.
    bytes32 public vKey;

    /// @notice The deployed SP1 verifier contract, handled by Succinct Labs.
    ISP1Verifier public verifier;

    // @notice The ABI encoded proof exposed outputs.
    struct ProofOutputs {
        uint64 trustedBlock;
        bytes32 trustedHeaderHash;
        uint64 targetBlock;
        bytes32 targetHeaderHash;
        bytes32 bridgeCommitment;
    }

    constructor(uint64 _height, bytes32 _headerHash, bytes32 _vKey, address _verifier) {
        latestBlock = _height;
        blockHeightToHeaderHash[_height] = _headerHash;
        vKey = _vKey;
        verifier = ISP1Verifier(_verifier);

        state_proofNonce = 1;
    }

    /// @notice Only the guardian can update the genesis state of the light client.
    function updateGenesisState(uint32 _height, bytes32 _header) external {
        blockHeightToHeaderHash[_height] = _header;
        latestBlock = _height;
    }

    /// @notice Only the guardian can update the verifier contract.
    function updateVerifier(address _verifier) external {
        verifier = ISP1Verifier(_verifier);
    }

    /// @notice Only the guardian can update the program vkey.
    function updateProgramVkey(bytes32 _programVkey) external {
        vKey = _programVkey;
    }

    /// @notice Commits the new header at targetBlock and the data commitment for the block range
    /// [latestBlock, targetBlock).
    /// @param proof The proof bytes for the SP1 proof.
    /// @param publicValues The public commitments from the SP1 proof.
    function commitHeaderRange(bytes calldata proof, bytes calldata publicValues) external {
        // Parse the outputs from the committed public values associated with the proof.
        ProofOutputs memory po = abi.decode(publicValues, (ProofOutputs));

        // Proof must be linked to the current latest block in the contract.
        bytes32 trustedHeader = blockHeightToHeaderHash[latestBlock];
        if (trustedHeader == bytes32(0)) {
            revert TrustedHeaderNotFound();
        }
        if (trustedHeader != po.trustedHeaderHash) {
            revert TrustedHeaderMismatch();
        }
        if (po.targetBlock <= latestBlock || po.targetBlock - latestBlock > BRIDGE_COMMITMENT_MAX) {
            revert TargetBlockNotInRange();
        }

        // Verify the proof with the associated public values. This will revert if proof invalid.
        verifier.verifyProof(vKey, publicValues, proof);

        // Store the new header and bridge commitment, and update the latest block and event nonce.
        blockHeightToHeaderHash[po.targetBlock] = po.targetHeaderHash;
        state_bridgeCommitments[state_proofNonce] = po.bridgeCommitment;
        state_proofNonce++;
        latestBlock = po.targetBlock;

        emit HeadUpdate(po.targetBlock, po.targetHeaderHash);
        emit BridgeCommitmentStored(
            state_proofNonce, po.trustedBlock, po.targetBlock, po.bridgeCommitment
        );
    }
}
