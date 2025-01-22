// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use edgeless_http::*;

struct HttpReadParameters;

impl EdgeFunction for HttpReadParameters {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {
        log::info!("http_read_parameters cast");
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("http_read_parameters: 'Call' called, MSG: {}", str_message);
        let req: EdgelessHTTPRequest = edgeless_http::request_from_string(&str_message).unwrap();

        let res_params = if req.path == "/calc_fractal" {
            if let Some(body) = req.body {
                if let Ok(content) = String::from_utf8(body) {
                    let tokens: Vec<&str> = content.split(",").collect();
                    if tokens.len() != 6 {
                        log::error!("expected exactly 6 tokens in input string, but got {}", tokens.len());
                        (400, Some(Vec::<u8>::from("input does not contain 6 elements")))
                    } else {
                        if tokens[0].parse::<usize>().is_ok() && tokens[1].parse::<usize>().is_ok() {
                            if tokens[2].parse::<f64>().is_ok()
                                && tokens[3].parse::<f64>().is_ok()
                                && tokens[4].parse::<f64>().is_ok()
                                && tokens[5].parse::<f64>().is_ok()
                            {
                                cast("parameters", &content.as_bytes());
                                (200, None)
                            } else {
                                log::error!("error parsing elements #3 - #6 in input string; one or more is not a float value");
                                (400, Some(Vec::<u8>::from("failed to parse float elements in input string")))
                            }
                        } else {
                            log::error!("first or second element in input string is not an int");
                            (400, Some(Vec::<u8>::from("failed to parse int elements in input string")))
                        }
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
        log::info!("http_read_parameters: 'Init' called");
    }

    fn handle_stop() {
        log::info!("http_read_parameters: 'Stop' called");
    }
}

edgeless_function::export!(HttpReadParameters);
