use edgeless_function::api::*;

struct HttpWriteNumberFun;

impl Edgefunction for HttpWriteNumberFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("http_write_number: 'Cast' called, MSG: {}", encoded_message);

        let res = call_alias(
            &"external_sink",
            &edgeless_http::request_to_string(&edgeless_http::EdgelessHTTPRequest {
                protocol: edgeless_http::EdgelessHTTPProtocol::HTTP,
                host: "localhost:10000".to_string(),
                headers: std::collections::HashMap::<String, String>::new(),
                body: Some(encoded_message.as_bytes().to_vec()),
                method: edgeless_http::EdgelessHTTPMethod::Post,
                path: "".to_string(),
            }),
        );

        if let edgeless_function::api::CallRet::Reply(response) = res {
            let parsed: edgeless_http::EdgelessHTTPResponse = edgeless_http::response_from_string(&response).unwrap();
            log::info!("HTTP_requestor: {:?}", std::str::from_utf8(&parsed.body.unwrap()));
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(_payload: String, _serialized_state: Option<String>) {
        log::info!("http_write_number: 'Init' called");
    }

    fn handle_stop() {
        log::info!("http_write_number: 'Stop' called");
    }
}

edgeless_function::export!(HttpWriteNumberFun);
