// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

// CheckTemperatureFun: Demo function to check if a temperature received is higher than 60 or lower than 40.
// If this is the case an output mapping "move_arm" is called with a command with
// parameters as defined by a struct MoveArmCallData

use dda;
use edgeless_function::*;
struct CheckTemperatureFun;

impl EdgeFunction for CheckTemperatureFun {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        log::info!("check_temperature_fun: cast called");
        let data = match dda::parse(encoded_message) {
            dda::DDA::ComSubscribeEvent(event_data) => String::from_utf8(event_data).expect("this should never fail"),
            _ => {
                log::error!("this should never happen");
                return;
            }
        };

        // Parse the UTF-8 string into a f64
        let trimmed_str = data.trim();
        let current_temperature = match trimmed_str.parse::<f64>() {
            Ok(value) => value,
            Err(_e) => {
                log::error!("check_temperature_fun: could not cast event data to float");
                return;
            }
        };

        if current_temperature > 60.0 {
            log::info!("check_temperature_fun: It's higher than 60 --> too hot! We need to take action -> forward event to move robotic arm function! Current temperature: {}", current_temperature);
            let move_cmd = format!("checktemp:{}:{}", "diff".to_string(), (current_temperature - 60.0).to_string());
            match call("move_arm", move_cmd.as_bytes()) {
                CallRet::NoReply => panic!("should never happen"),
                CallRet::Reply(res) => log::info!("all good!"),
                CallRet::Err => log::error!("calling function to move_arm did not work"),
            };
        } else if current_temperature < 40.0 {
            log::info!("check_temperature_fun: It's lower than 40 --> too cold! We need to take action -> forward event to move robotic arm function! Current temperature: {}", current_temperature);
            let move_cmd = format!("checktemp:{}:{}", "diff".to_string(), (current_temperature - 40.0).to_string());
            match call("move_arm", move_cmd.as_bytes()) {
                CallRet::NoReply => panic!("should never happen"),
                CallRet::Reply(res) => log::info!("all good!"),
                CallRet::Err => log::error!("calling function to move_arm did not work"),
            };
        } else {
            log::info!(
                "CheckTemperatureFun: Temperature is fine - no action needed!Current temperature: {}",
                current_temperature
            );
            return;
        }
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        log::info!("check_temperature_fun: handle_call should never be called!");
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("check_temperature_fun: 'Init' called. Wait for invocation by cast with DDA Event with temperature from resource!");
    }

    fn handle_stop() {
        log::info!("check_temperature_fun: 'Stop' called")
    }
}

edgeless_function::export!(CheckTemperatureFun);
