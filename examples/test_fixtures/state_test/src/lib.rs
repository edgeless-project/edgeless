use edgeless_function::api::*;
use log;

struct StateTest;

impl Edgefunction for StateTest {
    fn handle_cast(src: Fid, encoded_message: String) {
        match encoded_message.as_str() {
            "test_cast_output" => {
                sync("new_state");
            },
            _ => {
                log::info!("Unprocessed Message");
            }
        }
    }

    fn handle_call(src: Fid, encoded_message: String) -> CallRet {
        CallRet::Noreply

    }

    fn handle_init(payload: String, serialized_state: Option<String>) {
        edgeless_function::init_logger();
        if let Some(state) = serialized_state {
            log::info!("{}", state);
        } else {
            log::info!("{}", "no_state");
        }
    }

    fn handle_stop() {

    }
}

edgeless_function::export!(StateTest);
