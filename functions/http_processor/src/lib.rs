// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use edgeless_http::*;

struct ProcesorFun;

impl EdgeFunction for ProcesorFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        log::info!("HTTP_Processor: 'Cast' called, MSG: {:?}", encoded_message);
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::error!("Hello???");
        log::info!("HTTP_Processor: 'Call' called, MSG: {}", str_message);
        let req: EdgelessHTTPRequest = edgeless_http::request_from_string(str_message).unwrap();

        let resp = if req.path == "/hello" {
            EdgelessHTTPResponse {
                status: 200,
                body: Some(Vec::<u8>::from("World")),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        } else {
            EdgelessHTTPResponse {
                status: 404,
                body: Some(Vec::<u8>::from("Not Found")),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        };

        CallRet::Reply(OwnedByteBuff::new_from_slice(edgeless_http::response_to_string(&resp).as_bytes()))
    }

    fn handle_init(_payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("HTTP_Processor: 'Init' called");
    }

    fn handle_stop() {
        log::info!("HTTP_Processor: 'Stop' called");
    }
}

edgeless_function::export!(ProcesorFun);
