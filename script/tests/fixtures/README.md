## Ethereum

The following are the abi encoded function selectors that are used in the tests as fixtures when calling `eth_call`:

| Function name                   | Function ABI Encoded |
| ------------------------------- | -------------------- |
| latestBlock                     | 0x07e2da96           |
| BRIDGE_COMMITMENT_MAX           | 0x382f7579           |
| nonce (safe)                    | 0xaffed0e0           |
| usedNonce                       | 0xe4f376f0           |
| timeToFinalize                  | 0x4de5383b           |
| blockHeightToHeaderHash         | 0x08e93ea5           |
| state_bridgeCommitments         | 0xcdecf045           |
| commitmentTimestamp             | 0xfb6162b0           |
| processSequencerSupplyUpdate    | 0x2290bf58           |
| Aggregate3 (multicall contract) | 0x82ad56cb           |
| vKey                            | 0x625baeab           |

`eth_call`:

```command
curl -X POST -H "Content-Type: application/json" \
--data '{
  "jsonrpc": "2.0",
  "method": "eth_call",
  "params": [{"to": "0xc794851C58e7e5Bc6bd3A2b2785E0758BBEDea9F", "data": "0x2290bf58000000000000000000000000000000000000000000000000000000000000000b000000000000000000000000000000000000000000000000000000000000a8c10c"}, "latest"],
  "id": 1
}' \
https://rpc.ankr.com/eth_sepolia
```