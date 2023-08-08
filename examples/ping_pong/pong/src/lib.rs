use edgeless_function::api::*;
struct PongerFun;

impl Edgefunction for PongerFun {
    fn handle_cast(_src: Fid, encoded_message: String) {
        log::info!("Ponger: 'Cast' called, MSG: {}", encoded_message);
    }

    fn handle_call(_src: Fid, encoded_message: String) -> CallRet {
        log::info!("Ponger: 'Call' called, MSG: {}", encoded_message);
        CallRet::Reply("PONG".to_string())
    }

    fn handle_init(_payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("Ponger: 'Init' called");
    }

    fn handle_stop() {
        log::info!("Ponger: 'Stop' called");
    }
}
edgeless_function::export!(PongerFun);
