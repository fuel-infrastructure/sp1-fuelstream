#[cfg(test)]
pub mod tests {
    use crate::common::tests::load_mock_response;

    use core::panic;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Spawn another thread for the rpc server
    #[cfg(test)]
    pub async fn spawn_tendermint_rpc_server(fixture_name: String) -> String {
        let server = MockServer::start().await;

        // Http server simply returns the loaded json
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(move |req: &wiremock::Request| {
                let body_str = std::str::from_utf8(&req.body).unwrap();
                let body: serde_json::Value = serde_json::from_str(body_str).unwrap();
                let method = body["method"].as_str().unwrap_or_default();

                match method {
                    "status" => ResponseTemplate::new(200)
                        .set_body_json(load_mock_response(&fixture_name, "status.json")),
                    "commit" => {
                        let height = body["params"]["height"].as_str().unwrap_or("0");
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!("commit?height={}.json", height),
                        ))
                    }
                    "validators" => {
                        let height = body["params"]["height"].as_str().unwrap_or("0");
                        ResponseTemplate::new(200).set_body_json(load_mock_response(
                            &fixture_name,
                            &format!("validators?height={}.json", height),
                        ))
                    }
                    _ => panic!("unknown method received, method: {}, {}", method, body_str),
                }
            })
            .mount(&server)
            .await;

        format!("http://{}", server.address())
    }
}
