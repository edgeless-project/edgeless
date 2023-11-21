use edgeless_function::api::*;

struct Counter;

impl Edgefunction for Counter {
    fn handle_cast(_src: InstanceId, message: String) {
        let prev_count = message.parse::<i32>().unwrap();
        let cur_count = format!("{}", prev_count + 1);
        cast("output", cur_count.as_str());
        delayed_cast_raw(1000, &slf(), &cur_count);
    }

    fn handle_call(_src: InstanceId, _message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(init_message: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        let message = match init_message.parse::<i32>() {
            Ok(_) => init_message,
            Err(_) => "0".to_string(),
        };
        cast_raw(&slf(), &message);
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(Counter);
