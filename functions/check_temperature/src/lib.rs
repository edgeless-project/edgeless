// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

// CheckTemperatureFun: Demo function to check if a temperature received is higher than 60 or lower than 40. 
// If this is the case an output mapping "move_arm" is called with a command with 
// parameters as defined by a struct MoveArmCallData   

use edgeless_function::*;
use std::{process, str};
struct CheckTemperatureFun;

impl EdgeFunction for CheckTemperatureFun {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        let utf8_str = match str::from_utf8(encoded_message) {
            Ok(s) => s,
            Err(_e) => {process::exit(1);}
        };
    
        // Parse the UTF-8 string into a f64
        let trimmed_str = utf8_str.trim();
        let current_temperature = match trimmed_str.parse::<f64>() {
            Ok(value) => value,
            Err(_e) => {process::exit(1);}
        };    

        if current_temperature > 60.0 {
            log::info!("CheckTemperatureFun: It's higher than 60 --> too hot! We need to take action -> forward event to move robotic arm function! Current temperature: {}", current_temperature);
            let move_cmd = format!("checktemp:{}:{}", "diff".to_string(), (current_temperature - 60.0).to_string());
            let move_cmdb = move_cmd.as_bytes();
            // Use cast to avoid blocking of "call" as this has to go round-trip via the dda. 
            let _move_arm_result = cast("move_arm", move_cmdb);

        } else if current_temperature < 40.0 {
            log::info!("CheckTemperatureFun: It's lower than 40 --> too cold! We need to take action -> forward event to move robotic arm function! Current temperature: {}", current_temperature);
            let move_cmd = format!("checktemp:{}:{}", "diff".to_string(), (current_temperature- 40.0).to_string() );
            let move_cmdb = move_cmd.as_bytes();
            // Use cast to avoid blocking of "call" as this has to go round-trip via the dda. 
            let _move_arm_result = cast("move_arm", move_cmdb);
        } else{
            log::info!("CheckTemperatureFun: Temperature is fine - no action needed!Current temperature: {}", current_temperature);
        }
    }

    fn handle_call(_source: InstanceId, _encoded_message: &[u8]) -> CallRet {
        log::info!("CheckTemperatureFun: handle_call should never be called!");
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("CheckTemperatureFun: 'Init' called. Wait for invocation by cast with temperature from resource!");
    }

    fn handle_stop() {
        log::info!("CheckTemperatureFun: 'Stop' called")
    }
}

edgeless_function::export!(CheckTemperatureFun);


