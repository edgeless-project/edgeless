// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct CheckTemperatureFun;

impl EdgeFunction for CheckTemperatureFun {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        if encoded_message == b"routine_temperature_check" {
            log::info!("CheckTemperatureFun: Starting the routine temperature check");
            // call the dda to check the temperature
            let temperature_readings = call("dda", b"check_temperature");
            match temperature_readings {
                CallRet::Reply(msg) => match std::str::from_utf8(&msg) {
                    Ok("too_hot") => {
                        log::info!("CheckTemperatureFun: It's too hot! We need to move the robotic arm!");
                        let _move_arm_result = call("move_arm", b"hello, can you please move the robotic arm?");
                        // log::info!("CheckTemperatureFun: result {}", move_arm_result)
                        log::info!("CheckTemperatureFun: result ToDo");
                    }
                    Ok(_) => {
                        log::info!("CheckTemperatureFun: We don't need to move the robotic arm - the temperature is fine")
                    }
                    Err(_) => log::info!("CheckTemperatureFun: Received invalid UTF-8 data"),
                },
                CallRet::NoReply => log::info!("dda noreply"),
                CallRet::Err => log::info!("dda err"),
            }
            // Periodically, every 5 seconds invokes itself
            delayed_cast(5000, "self", b"routine_temperature_check")
        }
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        // TODO: idea for a task: connect http_ingress to this to be able to
        // explicitly trigger this function from the outside world
        log::info!("CheckTemperatureFun: handle_call should never be called!");
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        // Periodically, every 5 seconds invokes itself        edgeless_function::init_logger();
        log::info!("CheckTemperatureFun: 'Init' called. It will invoke itself every 5 seconds to check for too high temperatures");

        // Inside of a function we can also call outputs that are not explicitly
        // specified in the function.json / workflow.json file
        cast("self", b"routine_temperature_check");
    }

    fn handle_stop() {
        log::info!("CheckTemperatureFun: 'Stop' called")
    }
}

edgeless_function::export!(CheckTemperatureFun);


