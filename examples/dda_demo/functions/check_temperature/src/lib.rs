// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_function::api::*;

struct CheckTemperatureFun;

impl Edgefunction for CheckTemperatureFun {
    fn handle_cast(_source: InstanceId, msg: String) {
        if msg == "routine_temperature_check" {
            log::info!("CheckTemperatureFun: Starting the routine temperature check");
            // call the dda to check the temperature
            let temperature_readings = call("dda", "check_temperature");
            match temperature_readings {
                CallRet::Reply(msg) => match msg.as_str() {
                    "too_hot" => {
                        log::info!("It's too hot! We need to move the robotic arm!");
                        let move_arm_result = call("move_arm", "hello, can you please move the robotic arm?");
                    }
                    _ => {
                        log::info!("We don't need to move the robotic arm - the temperature is fine")
                    }
                },
                CallRet::Noreply => log::info!("dda noreply"),
                CallRet::Err => log::info!("dda err"),
            }

            delayed_cast(5000, "self", "routine_temperature_check")
        }
    }

    fn handle_call(_source: InstanceId, _msg: String) -> CallRet {
        // TODO: idea for a task: connect http_ingress to this to be able to
        // explicitly trigger this function from the outside world
        log::info!("This should never be called!");
        CallRet::Noreply
    }

    fn handle_init(_payload: String, _state: Option<String>) {
        // Periodically, every 5 seconds invokes itself
        edgeless_function::init_logger();
        log::info!("CheckTemperatureFun: 'Init' called. It will invoke itself every 5 seconds to check for too high temperatures");

        // Inside of a function we can also call outputs that are not explicitly
        // specified in the function.json / workflow.json file
        cast("self", "routine_temperature_check");
    }

    fn handle_stop() {
        log::info!("CheckTemperatureFun: 'Stop' called")
    }
}

edgeless_function::export!(CheckTemperatureFun);
