#[cfg(test)]
pub mod tests {
    use crate::common::tests::load_mock_response;

    use core::panic;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Spawn another thread for the rpc server
    #[cfg(test)]
    pub async fn spawn_ethereum_rpc_server(fixture_name: String) -> String {
        let server = MockServer::start().await;

        // Http server simply returns the loaded json
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                let body_str = std::str::from_utf8(&req.body).unwrap();
                let body: serde_json::Value = serde_json::from_str(body_str).unwrap();
                let method = body["method"].as_str().unwrap_or_default();

                match method {
                    "eth_call" => {
                        let params = &body["params"][0];
                        let contract = params["to"].as_str().unwrap_or_default();
                        let function_call = params["input"].as_str().unwrap_or_default();
                        // Truncate to avoid file length issues
                        let truncated_call = &function_call[..function_call.len().min(140)];
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!("eth_call?to={}&data={}.json", contract, truncated_call),
                        ))
                    }
                    "eth_getCode" => {
                        let address = body["params"][0].as_str().unwrap_or_default();
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!("eth_getCode?to={}.json", address),
                        ))
                    }
                    "eth_getTransactionCount" => {
                        let address = body["params"][0].as_str().unwrap_or_default();
                        let status = body["params"][1].as_str().unwrap_or_default();
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!(
                                "eth_getTransactionCount?address={}&status={}.json",
                                address, status
                            ),
                        ))
                    }
                    "eth_sendRawTransaction" => {
                        let tx = body["params"][0].as_str().unwrap_or_default();
                        let truncated_tx = &tx[..tx.len().min(140)];
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!("eth_sendRawTransaction?tx={}.json", truncated_tx),
                        ))
                    }
                    "eth_getTransactionReceipt" => {
                        let tx_hash = body["params"][0].as_str().unwrap_or_default();
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!("eth_getTransactionReceipt?txHash={}.json", tx_hash),
                        ))
                    }
                    "net_version" | "eth_chainId" | "eth_gasPrice" | "eth_blockNumber"
                    | "eth_estimateGas" => ResponseTemplate::new(200).set_body_json(
                        load_mock_response(&fixture_name, &format!("{}.json", method)),
                    ),
                    _ => panic!("unknown method received, method: {}, {}", method, body_str),
                }
            })
            .mount(&server)
            .await;

        format!("http://{}", server.address())
    }
}
