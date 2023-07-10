wit_bindgen::generate!("edgefun");

struct PongerFun;

impl Edgefun for PongerFun {
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

export_edgefun!(PongerFun);
