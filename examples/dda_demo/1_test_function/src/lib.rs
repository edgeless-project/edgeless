use edgeless_function::api::*;

struct TestFunction;

impl Edgefunction for TestFunction {
    fn handle_cast(_src: InstanceId, encoded_message: String) {}

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {}

    fn handle_stop() {}
}

edgeless_function::export!(TestFunction);
