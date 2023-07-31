use std::ops::Deref;

use edgeless_function::api::*;

struct PingerFun;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct PingerState {
    count: u64,
}

impl Edgefunction for PingerFun {
    fn handle_cast(_src: Fid, encoded_message: String) {
        log(&format!("AsyncPinger: 'Cast' called, MSG: {}", encoded_message));
        if encoded_message == "wakeup" {
            cast_alias("ponger", "PING");
            delayed_cast(1000, &slf(), "wakeup");
        } else {
            log("Got Response");
        }
    }

    fn handle_call(_src: Fid, encoded_message: String) -> CallRet {
        log(&format!("AsyncPinger: 'Call' called, MSG: {}", encoded_message));
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        log("AsyncPinger: 'Init' called");
        cast(&slf(), "wakeup");
    }

    fn handle_stop() {
        log("AsyncPinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
