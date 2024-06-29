use edgeless_function::*;
use edgeless_http::*;

struct Forward;

impl EdgeFunction for Forward {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        cast("out", message);
    }

    fn handle_call(_src: InstanceId, message: &[u8]) -> CallRet {
        let req: EdgelessHTTPRequest = edgeless_http::request_from_string(core::str::from_utf8(message).unwrap()).unwrap();

        cast("out", &req.body.unwrap_or(vec![]));

        let resp = EdgelessHTTPResponse {
            status: 200,
            body: None,
            headers: std::collections::HashMap::<String, String>::new(),
        };

        CallRet::Reply(OwnedByteBuff::new_from_slice(edgeless_http::response_to_string(&resp).as_bytes()))
    }

    fn handle_init(_init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {}

    fn handle_stop() {
        //noop
    }
}

edgeless_function::export!(Forward);
