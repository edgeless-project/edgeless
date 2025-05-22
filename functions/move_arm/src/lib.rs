// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use dda;
use edgeless_function::*;
use std::str;

// MoveArmFun: Demo function to take a received message, deserialize it to a struct
// MoveArmCallData. Based on the content a command and information is generated to
// be forwared dda resource with a specific command.

struct MoveArmFun;

impl EdgeFunction for MoveArmFun {
    fn handle_cast(_src: InstanceId, _encoded_message: &[u8]) {
        log::info!("MoveArmFun: a cast should never be called!");
    }

    // receives a simple dataplane call from the check_temperature function
    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("MoveArmFun: call");
        let msg_str = std::str::from_utf8(encoded_message).expect("wrong message from check_temperature_fun");

        let tokens: Vec<&str> = msg_str.split(':').collect();
        if tokens.len() == 3 && tokens[0] == "checktemp" {
            let _subcmd = tokens[1];
            let diff_value_str = tokens[2];
            let trimmed_str = diff_value_str.trim();
            let diff_value = match trimmed_str.parse::<f64>() {
                Ok(value) => value,
                Err(_e) => {
                    panic!("wrong message - can not cast to f64");
                }
            };

            // Generate a move value based on temperature diff
            let mov_para = diff_value * 0.1;
            let action_data = format!("{}", mov_para).as_bytes().to_vec();

            // publish an action over dda and wait for a response (can block
            // indefinitely)
            log::info!("publishing an action over dda");
            match dda::publish_action("move_arm", action_data) {
                Ok(res) => {
                    log::info!(
                        "move_arm DDA action success {}",
                        String::from_utf8(res.clone()).expect("should never happen")
                    );
                    CallRet::Reply(OwnedByteBuff::new_from_slice(res.as_slice()))
                }
                Err(e) => {
                    log::error!("move_arm DDA action did not succeed: {}", e);
                    CallRet::Err
                }
            }
        } else {
            log::info!("MoveArmFun: invalid message received on call()");
            CallRet::Err
        }
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
