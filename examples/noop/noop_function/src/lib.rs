use edgeless_function::api::*;
use log;

struct NoopFunction;

impl Edgefunction for NoopFunction {
    fn handle_cast(src: Fid, encoded_message: String) {
        log::info!("Noop casted, node {}, function {}, MSG: {}", src.node, src.function, encoded_message);
    }

    fn handle_call(src: Fid, encoded_message: String) -> CallRet {
        log::info!("Noop called, node {}, function {}, MSG: {}", src.node, src.function, encoded_message);
        CallRet::Noreply
    }

    fn handle_init(payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("Noop initialized, payload: {}", payload);
    }

    fn handle_stop() {
        log::info!("Noop stopped");
    }
}

edgeless_function::export!(NoopFunction);
