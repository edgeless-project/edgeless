// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use edgeless_http::*;

struct RequestorFun;

impl EdgeFunction for RequestorFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        log::info!("HTTP_Requestor: 'Cast' called, MSG: {:?}", encoded_message);

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
            })
            .as_bytes(),
        );

        if let edgeless_function::CallRet::Reply(response) = res {
            let parsed: edgeless_http::EdgelessHTTPResponse = edgeless_http::response_from_string(core::str::from_utf8(&response).unwrap()).unwrap();
            log::info!("HTTP_requestor: {:?}", std::str::from_utf8(&parsed.body.unwrap()));
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("HTTP_Requestor: 'Call' called, MSG: {:?}", encoded_message);
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("HTTP_Requestor: 'Init' called");
        delayed_cast(5000, "self", "wakeup".as_bytes());
    }

    fn handle_stop() {
        log::info!("HTTP_Requestor: 'Stop' called");
    }
}

edgeless_function::export!(RequestorFun);
