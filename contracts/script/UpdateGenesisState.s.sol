// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

// import "forge-std/Script.sol";
// import {FuelStreamX} from "../src/FuelStreamX.sol";
// import {ERC1967Proxy} from "@openzeppelin/proxy/ERC1967/ERC1967Proxy.sol";
// import {SP1MockVerifier} from "@sp1-contracts/SP1MockVerifier.sol";
// import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

// import {BaseScript} from "./Base.s.sol";

// // Required environment variables:
// // - GENESIS_HEIGHT
// // - GENESIS_HEADER
// // - CONTRACT_ADDRESS

// contract UpdateGenesisStateScript is BaseScript {
//     string internal constant KEY = "SP1_BLOBSTREAM";

//     function run() external multichain(KEY) returns (address) {
//         vm.startBroadcast();

//         SP1Blobstream lightClient = SP1Blobstream(vm.envAddress("CONTRACT_ADDRESS"));

//         lightClient.updateGenesisState(
//             uint32(vm.envUint("GENESIS_HEIGHT")), vm.envBytes32("GENESIS_HEADER")
//         );

//         return address(lightClient);
//     }
// }
