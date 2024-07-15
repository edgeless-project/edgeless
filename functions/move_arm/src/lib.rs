// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use std::{process, str};


// MoveArmFun: Demo function to take a received message, deserialize it to a struct
// MoveArmCallData. Based on the content a command and information is generated to 
// be forwared dda resource with a specific command.  

// Communication with the outside world (also with resources / other components)
// from an edgless function always happens explicitly over the dataplane calls
// call(), cast() - the first parameter identifies the target component. The
// second parameter is the stringified message that is sent to the other
// component.
// Right now it's all hard-coded in the dda resource definition!!!!

// TODO: import macros / library for dda binding - like in http_ingress / egress
// examples; allow to call dda resource directly from the edgeless function

struct MoveArmFun;

impl EdgeFunction for MoveArmFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let msg_str = std::str::from_utf8(encoded_message).expect("");

        let tokens: Vec<&str> = msg_str.split(':').collect();
        if tokens.len() == 3 && tokens[0] == "checktemp" {
            let _subcmd = tokens[1];
            let diff_value_str = tokens[2];
            let trimmed_str = diff_value_str.trim();
            let diff_value = match trimmed_str.parse::<f64>() {
                Ok(value) => value,
                Err(_e) => {process::exit(1);}
            }; 
            
            // Generate a move value based on temperature diff
            let mov_para = diff_value * 0.1;
            
            // Initial json kind of version to transport both a dda function to call and parameters
            let mut json_params = String::new();
            json_params.push_str(&format!(r#"{{"pubid": "dda_move_arm", "pattern": "action", "params": "{}"}}"#, mov_para.to_string()));
            let res = call("dda", json_params.as_bytes());

            if let CallRet::Reply(response) = res {
                match std::str::from_utf8(&response) {
                    Ok(s) => log::info!("MoveArmFun: moved arm over DDA with the following response {}", s),
                    Err(e) => log::info!("MoveArmFun: Received invalid UTF-8 data {}", e),
                }
            }
        } else {
            log::info!("MoveArmFun: invalid message received on cast()");
        }
    }

    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("Move Arm Function: handle_call called {:?} - but not implemented!", std::str::from_utf8(encoded_message));
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("MoveArmFun: 'Init' called");
    }

    fn handle_stop() {
        log::info!("MoveArmFun: 'Stop' called");
    }
}

edgeless_function::export!(MoveArmFun);
