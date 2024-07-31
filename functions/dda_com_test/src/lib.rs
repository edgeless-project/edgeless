// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;
use prost::*;

struct DDAComTest;

impl EdgeFunction for DDAComTest {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        log::info!("com: cast called");
        // incoming cast contains an event (as specified in workflow)
        let data = match dda::parse(encoded_message) {
            dda::DDA::ComSubscribeEvent(event_data) => String::from_utf8(event_data),
            _ => {
                log::error!("this should never happen");
                return;
            }
        };
        log::info!("successfully got an event over dda! {:?}", data);
        let event_str = String::from("event from edgeless function");
        // publishing is done over an alias defined in the workflow.json mapping
        match dda::publish_event("eve", event_str.encode_to_vec()) {
            Ok(_) => log::info!("publish okay"),
            Err(_) => log::error!("publish not okay"),
        }
    }

    fn handle_call(_source: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("com: call called");
        let (correlation_id, data) = match dda::parse(encoded_message) {
            dda::DDA::ComSubscribeAction(correlation_id, event) => (correlation_id, String::from_utf8(event)),
            _ => {
                log::error!("should never happen");
                return CallRet::Err;
            }
        };
        log::info!("successfully got an action over dda: {:?}. now responding", data);
        let result_data = String::from("action result").encode_to_vec();
        match dda::publish_action_result(correlation_id, result_data) {
            Ok(_) => log::info!("action result published"),
            Err(_) => log::error!("action result could not be published"),
        }

        // now publish an action and receive a single response
        // publishing is done over an alias defined in the workflow.json
        let action_payload = String::from("hello action");
        let action_result = dda::publish_action("act", action_payload.encode_to_vec());
        match action_result {
            Ok(res) => {
                log::info!("got an action result back: {:?}", String::from_utf8(res));
                CallRet::NoReply
            }
            Err(e) => {
                log::error!("got no action result back: reason={e}");
                CallRet::Err
            }
        }
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("com: 'Init' called. Waiting for actions through DDA!");
    }

    fn handle_stop() {
        log::info!("com: stop called");
    }
}

edgeless_function::export!(DDAComTest);
