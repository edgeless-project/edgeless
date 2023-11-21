use edgeless_function::api::*;

struct DoubleFun;

impl Edgefunction for DoubleFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {
        log::info!("double: called with '{}'", encoded_message);

        if let Ok(n) = encoded_message.parse::<i32>() {
            cast("result", format!("{}", 2 * n).as_str());
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(_payload: String, _init_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("double: started");
    }

    fn handle_stop() {
        log::info!("double: stopped");
    }
}

edgeless_function::export!(DoubleFun);
