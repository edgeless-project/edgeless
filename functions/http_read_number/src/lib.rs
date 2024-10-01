// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use edgeless_http::*;

struct HttpReadNumberFun;

impl EdgeFunction for HttpReadNumberFun {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {}

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        let str_message = core::str::from_utf8(encoded_message).unwrap();

        log::info!("http_read_number: 'Call' called, MSG: {}", str_message);
        let req: EdgelessHTTPRequest = edgeless_http::request_from_string(&str_message).unwrap();

        let res_params = if req.path == "/read_number" {
            if let Some(body) = req.body {
                if let Ok(content) = String::from_utf8(body) {
                    if let Ok(_) = content.parse::<i32>() {
                        cast("parsed_value", content.as_bytes());
                        (200, None)
                    } else {
                        (400, Some(Vec::<u8>::from("body does not contain an integer")))
                    }
                } else {
                    (400, Some(Vec::<u8>::from("body is not a string")))
                }
            } else {
                (400, Some(Vec::<u8>::from("empty body")))
            }
        } else {
            (404, Some(Vec::<u8>::from("invalid path")))
        };

        let res = EdgelessHTTPResponse {
            status: res_params.0,
            body: res_params.1,
            headers: std::collections::HashMap::<String, String>::new(),
        };

        CallRet::Reply(OwnedByteBuff::new_from_slice(edgeless_http::response_to_string(&res).as_bytes()))
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("http_read_number: 'Init' called");
    }

    fn handle_stop() {
        log::info!("http_read_number: 'Stop' called");
    }
}

edgeless_function::export!(HttpReadNumberFun);
