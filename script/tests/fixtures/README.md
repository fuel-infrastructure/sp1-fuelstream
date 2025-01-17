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
  "params": [{"to": "0x4e6111c3700cf93e2a3ac513020e49463a5327b5", "data": "0x07e2da96"}, latest],
  "id": 1
}' \
https://rpc.ankr.com/eth_sepolia
```