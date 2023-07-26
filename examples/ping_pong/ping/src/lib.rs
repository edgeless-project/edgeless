use std::ops::Deref;

use edgeless_function::api::*;

struct PingerFun;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct PingerState {
    count: u64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<PingerState>> = std::sync::OnceLock::new();

impl Edgefunction for PingerFun {
    fn handle_cast(_src: Fid, encoded_message: String) {
        log(&format!("Pinger: 'Cast' called, MSG: {}", encoded_message));
        if encoded_message == "wakeup" {
            // cast_alias("ponger", "PING");
            let id = STATE.get().unwrap().lock().unwrap().count;
            STATE.get().unwrap().lock().unwrap().count += 1;
            sync(&serde_json::to_string(STATE.get().unwrap().lock().unwrap().deref()).unwrap());
            let res = call_alias("ponger", &format!("PING-{}", id));
            if let CallRet::Reply(_msg) = res {
                log("Got Reply");
            }
            delayed_cast(1000, &slf(), "wakeup");
        }
    }

    fn handle_call(_src: Fid, encoded_message: String) -> CallRet {
        log(&format!("Pinger: 'Call' called, MSG: {}", encoded_message));
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        log("Pinger: 'Init' called");
        if let Some(serialized) = serialized_state {
            STATE.set(std::sync::Mutex::new(serde_json::from_str(&serialized).unwrap())).unwrap();
        } else {
            STATE.set(std::sync::Mutex::new(PingerState { count: 0 })).unwrap();
        }
        cast(&slf(), "wakeup");
    }

    fn handle_stop() {
        log("Pinger: 'Stop' called");
    }
}

edgeless_function::export!(PingerFun);
