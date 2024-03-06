// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct HttpWriteNumberFun;

static CONFIGURATION: std::sync::OnceLock<String> = std::sync::OnceLock::new();

impl EdgeFunction for HttpWriteNumberFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        log::info!("http_write_number: 'Cast' called, MSG: {:?}", encoded_message);

        let res = call(
            &"external_sink",
            &edgeless_http::request_to_string(&edgeless_http::EdgelessHTTPRequest {
                protocol: edgeless_http::EdgelessHTTPProtocol::HTTP,
                host: CONFIGURATION.get().unwrap().clone(),
                headers: std::collections::HashMap::<String, String>::new(),
                body: Some(encoded_message.to_vec()),
                method: edgeless_http::EdgelessHTTPMethod::Post,
                path: "".to_string(),
            })
            .as_bytes(),
        );

        if let CallRet::Reply(response) = res {
            let parsed: edgeless_http::EdgelessHTTPResponse = edgeless_http::response_from_string(core::str::from_utf8(&response).unwrap()).unwrap();
            log::info!("HTTP_requestor: {:?}", std::str::from_utf8(&parsed.body.unwrap()));
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        log::info!("http_write_number: 'Init' called");
        if let Some(payload) = payload {
            let payload_str = core::str::from_utf8(payload).unwrap();
            assert!(CONFIGURATION.set(payload_str.to_string()).is_ok());
        }
    }

    fn handle_stop() {
        log::info!("http_write_number: 'Stop' called");
    }
}

edgeless_function::export!(HttpWriteNumberFun);
