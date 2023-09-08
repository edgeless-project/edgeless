use edgeless_function::api::*;
use edgeless_http::*;

struct HttpReadNumberFun;

impl Edgefunction for HttpReadNumberFun {
    fn handle_cast(_src: InstanceId, _encoded_message: String) {}

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        log::info!("http_read_number: 'Call' called, MSG: {}", encoded_message);
        let req: EdgelessHTTPRequest = edgeless_http::request_from_string(&encoded_message).unwrap();

        let res = if req.path == "/read_number" {
            if let Some(body) = req.body {
                if let Ok(content) = String::from_utf8(body) {
                    if let Ok(_) = content.parse::<i32>() {
                        cast_alias("cb_success", &content);
                        EdgelessHTTPResponse {
                            status: 200,
                            body: None,
                            headers: std::collections::HashMap::<String, String>::new(),
                        }
                    } else {
                        EdgelessHTTPResponse {
                            status: 400,
                            body: Some(Vec::<u8>::from("body does not contain an integer")),
                            headers: std::collections::HashMap::<String, String>::new(),
                        }
                    }
                } else {
                    EdgelessHTTPResponse {
                        status: 400,
                        body: Some(Vec::<u8>::from("body is not a string")),
                        headers: std::collections::HashMap::<String, String>::new(),
                    }
                }
            } else {
                EdgelessHTTPResponse {
                    status: 400,
                    body: Some(Vec::<u8>::from("empty body")),
                    headers: std::collections::HashMap::<String, String>::new(),
                }
            }
        } else {
            EdgelessHTTPResponse {
                status: 404,
                body: Some(Vec::<u8>::from("invalid path")),
                headers: std::collections::HashMap::<String, String>::new(),
            }
        };

        CallRet::Reply(edgeless_http::response_to_string(&res))
    }

    fn handle_init(_payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("http_read_number: 'Init' called");
    }

    fn handle_stop() {
        log::info!("http_read_number: 'Stop' called");
    }
}

edgeless_function::export!(HttpReadNumberFun);
