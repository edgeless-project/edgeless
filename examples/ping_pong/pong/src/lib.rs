use edgeless_function::api::*;
struct PongerFun;

impl Edgefunction for PongerFun {
    fn handle_call(src: Fid, encoded_message: String) {
        log(&format!("Ponger: 'Call' called, MSG: {}", encoded_message));
        call(&src, "PONG");
    }

    fn handle_init(_payload: String) {
        log("Ponger: 'Init' called");
    }

    fn handle_stop() {
        log("Ponger: 'Stop' called");
    }
}
    edgeless_function::export!(PongerFun);
