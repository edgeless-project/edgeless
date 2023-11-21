use edgeless_function::api::*;
use edgeless_http::*;

struct RequestorFun;

impl Edgefunction for RequestorFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("HTTP_Requestor: 'Cast' called, MSG: {}", encoded_message);

        let res = call(
            &"http_e",
            &edgeless_http::request_to_string(&edgeless_http::EdgelessHTTPRequest {
                protocol: edgeless_http::EdgelessHTTPProtocol::HTTPS,
                host: "api.github.com:443".to_string(),
                headers: std::collections::HashMap::<String, String>::from([
                    ("Accept".to_string(), "application/vnd.github+json".to_string()),
                    ("User-Agent".to_string(), "edgeless".to_string()),
                ]),
                body: None,
                method: edgeless_http::EdgelessHTTPMethod::Get,
                path: "/users/raphaelhetzel/keys".to_string(),
            }),
        );

        if let edgeless_function::api::CallRet::Reply(response) = res {
            let parsed: edgeless_http::EdgelessHTTPResponse = edgeless_http::response_from_string(&response).unwrap();
            log::info!("HTTP_requestor: {:?}", std::str::from_utf8(&parsed.body.unwrap()));
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        log::info!("HTTP_Requestor: 'Call' called, MSG: {}", encoded_message);
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        log::info!("HTTP_Requestor: 'Init' called");
        delayed_cast(5000, "self", "wakeup");
    }

    fn handle_stop() {
        log::info!("HTTP_Requestor: 'Stop' called");
    }
}

edgeless_function::export!(RequestorFun);
