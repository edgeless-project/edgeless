use edgeless_function::api::*;
use edgeless_http::*;


struct ProcesorFun;

impl Edgefunction for ProcesorFun {
    fn handle_cast(_src: Fid, encoded_message: String) {
        log(&format!("HTTP_Processor: 'Cast' called, MSG: {}", encoded_message));
    }

    fn handle_call(_src: Fid, encoded_message: String) -> CallRet {
        log(&format!("HTTP_Processor: 'Call' called, MSG: {}", encoded_message));
        let req : EdgelessHTTPRequest = edgeless_http::request_from_string(&encoded_message).unwrap();

        let resp = if req.path == "/hello" {
            EdgelessHTTPResponse {
                status: 200,
                body: Some(Vec::<u8>::from("World")),
                headers: std::collections::HashMap::<String, String>::new()
            }
        } else {
            EdgelessHTTPResponse {
                status: 404,
                body: Some(Vec::<u8>::from("Not Found")),
                headers: std::collections::HashMap::<String, String>::new()
            }
        };

        CallRet::Reply(edgeless_http::response_to_string(&resp))
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        log("HTTP_Processor: 'Init' called");
    }

    fn handle_stop() {
        log("HTTP_Processor: 'Stop' called");
    }
}

edgeless_function::export!(ProcesorFun);
