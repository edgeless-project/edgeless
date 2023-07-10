use edgeless_function::api::*;

struct PingerFun;

impl Edgefunction for PingerFun {
    fn handle_call(_src: Fid, encoded_message: String) {
        log(&format!("Pinger: 'Call' called, MSG: {}", encoded_message));
        if encoded_message == "wakeup" {
            call_alias("ponger", "PING");
            delayed_call(1000, &slf(), "wakeup");
        }
    }

    fn handle_init(_payload: String) {
        log("Pinger: 'Init' called");
        call(&slf(), "wakeup");
    }

    fn handle_stop() {
        log("Pinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
