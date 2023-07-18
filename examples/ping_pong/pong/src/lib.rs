use edgeless_function::api::*;
struct PongerFun;

impl Edgefunction for PongerFun {
    fn handle_cast(_src: Fid, encoded_message: String) {
        log(&format!("Ponger: 'Cast' called, MSG: {}", encoded_message));
        // call(&src, "PONG");
        cast_alias("pinger", "PONG2");
    }

    fn handle_call(_src: Fid, encoded_message: String) -> CallRet {
        // log("Ponger: 'Call' called");
        log(&format!("Ponger: 'Call' called, MSG: {}", encoded_message));
        CallRet::Reply("PONG3".to_string())
    }

    fn handle_init(_payload: String, _serialized_state: Option<String>) {
        log("Ponger: 'Init' called");
    }

    fn handle_stop() {
        log("Ponger: 'Stop' called");
    }
}
edgeless_function::export!(PongerFun);
