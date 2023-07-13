use edgeless_function::api::*;

struct PingerFun;

impl Edgefunction for PingerFun {
    fn handle_cast(_src: Fid, encoded_message: String) {
        log(&format!("Pinger: 'Cast' called, MSG: {}", encoded_message));
        if encoded_message == "wakeup" {
            // cast_alias("ponger", "PING");
            let res = call_alias("ponger", "PING3");
            if let CallRet::Reply(_msg) = res {
                log("Got Reply");
            }
            delayed_cast(1000, &slf(), "wakeup");
        }
    }

    fn handle_call(_src: Fid, _encoded_message: String) -> CallRet {
        log("Ponger: 'Call' called");
        CallRet::Noreply
    }

    fn handle_init(_payload: String) {
        log("Pinger: 'Init' called");
        cast(&slf(), "wakeup");
    }

    fn handle_stop() {
        log("Pinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
