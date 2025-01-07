// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {Script, console} from "forge-std/Script.sol";

import {FuelStreamX} from "../src/FuelStreamX.sol";
import {ERC1967Proxy} from "@openzeppelin/proxy/ERC1967/ERC1967Proxy.sol";
import {SP1MockVerifier} from "@sp1-contracts/SP1MockVerifier.sol";
import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

// Required environment variables:
// - GENESIS_HEIGHT
// - GENESIS_HEADER
// - VKEY

contract DeployWithVerifierScript is Script {
    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        // The verifier, handled by Succinct Labs
        ISP1Verifier verifier = ISP1Verifier(vm.envAddress("SP1_VERIFIER_ADDRESS"));

        // Deploy the FuelStreamX contract.
        uint64 trustedHeight = uint64(vm.envUint("GENESIS_HEIGHT"));
        bytes32 trustedBlockHeader = bytes32(vm.envBytes("GENESIS_HEADER"));
        bytes32 vkey = bytes32(vm.envBytes("VKEY"));

        FuelStreamX fuelStreamX =
            new FuelStreamX(trustedHeight, trustedBlockHeader, vkey, address(verifier));

        console.log("FUELSTREAMX deployed at: ");
        console.logAddress(address(fuelStreamX));

        vm.stopBroadcast();
    }
}
