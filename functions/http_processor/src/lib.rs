// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use edgeless_http::*;

struct ProcessorFun;

edgeless_function::generate!(ProcessorFun);

impl HttpProcessorAPI for ProcessorFun {

    type EDGELESS_HTTP_REQUEST = edgeless_http::EdgelessHTTPRequest;
    type EDGELESS_HTTP_RESPONSE = edgeless_http::EdgelessHTTPResponse;

    fn handle_call_new_req(_src: InstanceId, req: EdgelessHTTPRequest) -> EdgelessHTTPResponse {
        log::info!("HTTP_Processor: 'Call' called, MSG: {:?}", req);

        if req.path == "/hello" {
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
        }
    }

    fn handle_internal(_: &[u8]) {}

    fn handle_init(_payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        log::info!("HTTP_Processor: 'Init' called");
    }

    fn handle_stop() {
        log::info!("HTTP_Processor: 'Stop' called");
    }
}
