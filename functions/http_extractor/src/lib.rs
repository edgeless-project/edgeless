// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_function::*;
use edgeless_http::*;

struct HttpExtractor;

impl EdgeFunction for HttpExtractor {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {}

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        if let Some(body) = edgeless_http::request_from_string(core::str::from_utf8(encoded_message).unwrap())
            .unwrap()
            .body
        {
            cast("out", &body);
        }

        CallRet::Reply(OwnedByteBuff::new_from_slice(
            edgeless_http::response_to_string(&EdgelessHTTPResponse {
                status: 200,
                body: None,
                headers: std::collections::HashMap::<String, String>::new(),
            })
            .as_bytes(),
        ))
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {}

    fn handle_stop() {}
}

edgeless_function::export!(HttpExtractor);
