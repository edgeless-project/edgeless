use edgeless_function::api::*;

struct CheckTemperatureFun;

impl Edgefunction for CheckTemperatureFun {
    fn handle_cast(_source: InstanceId, msg: String) {
        log::info!("CheckTemperatureFun: 'Cast' called with msg={}", msg);
        if msg == "routine_temperature_check" {
            log::info!("calling dda");
            // TODO: call of dda is blocking - dataplane event is never received
            // by the dda singleton - something is wrong in the configuration?
            // -> write a simple workflow with just one call to the dda, compare
            // dda configuration to http_ingress configuration
            let temperature_readings = call("dda", "read_temperature"); // TODO: how do we pass a parameter to the read_temperature action?
            match temperature_readings {
                CallRet::Reply(msg) => log::info!("returned {}", msg),
                CallRet::Noreply => log::info!("dda noreply"),
                CallRet::Err => log::info!("dda err"),
            }
            let _hello = cast("output", "hello");
            let _log = cast("log_output", "test");
            log::info!("after dda call");
            // TODO: trigger move_arm function
        }
    }

    fn handle_call(_source: InstanceId, _msg: String) -> CallRet {
        // TODO: idea for a task: connect http_ingress to this to be able to
        // explicitly trigger this function from the outside world
        log::info!("This should never be called!");
        CallRet::Noreply
    }

    fn handle_init(_payload: String, _state: Option<String>) {
        // Periodically, every 10 seconds invokes itself
        edgeless_function::init_logger();
        log::info!("CheckTemperatureFun: 'Init' called");

        // Inside of a function we can also call outputs that are not explicitly
        // specified in the function.json / workflow.json file
        cast("self", "routine_temperature_check");
    }

    fn handle_stop() {
        log::info!("CheckTemperatureFun: 'Stop' called")
    }
}

edgeless_function::export!(CheckTemperatureFun);
