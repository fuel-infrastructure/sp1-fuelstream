## Sequencer

To obtain the fixtures for the sequencer, you can use the normal tendermint RPC calls, usually on port 26657.

To obtain the fixtures for grpc queries, you can use the swagger api as follows `<REST>/swagger/#/Fuel%20Sequencer%20Queries/BridgeCommitment`.

## Ethereum

The following are the abi encoded function selectors that are used in the tests as fixtures when calling `eth_call`:

| Function name                   | Function ABI Encoded |
| ------------------------------- | -------------------- |
| BRIDGE_COMMITMENT_MAX           | 0x382f7579           |
| latestBlock                     | 0x07e2da96           |
| blockHeightToHeaderHash         | 0x08e93ea5           |
| vKey                            | 0x625baeab           |

To obtain any of the above, the following curl command with minor changes to the `method` and `params`. 
For example `eth_call`:

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