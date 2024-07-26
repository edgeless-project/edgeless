// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;
struct DDAComTest;

impl EdgeFunction for DDAComTest {
    // Following logic is implemented: this function subscribes to incoming
    // events, actions and queries. For event it receives it and sends a
    // response event out. For action in receives it, sends a result (through
    // ActionResult) and then starts another action and waits for it result. For
    // Query the procedure is the same as for Action. Through that we are able
    // to cover all of the methods of DDA.
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        log::info!("dda_com: handle cast");
        match dda::parse(encoded_message) {
            dda::DDA::ComSubscribeEvent(event_data) => {
                log::info!("event subscribe");
                // send an event out
                match dda::publish_event("event_alias", vec![]) {
                    Ok(_) => log::info!("event publish"),
                    Err(_) => log::error!("event publish error"),
                }
            }
            dda::DDA::ComSubscribeAction(correlation_id, action_data) => {
                log::info!("action subscribe");
                let result_data = vec![];
                match dda::publish_action_result(correlation_id, result_data) {
                    Ok(_) => log::info!("action result publish"),
                    Err(_) => log::error!("action result publish error"),
                };

                // send a custom action and wait for the result
                match dda::publish_action("action_alias", vec![]) {
                    Ok(_) => log::info!("action publish"),
                    Err(_) => log::error!("action publish error"),
                }
            }
            dda::DDA::ComSubscribeQuery(correlation_id, query_data) => {
                log::info!("query subscribe");
                match dda::publish_query_result(correlation_id, vec![]) {
                    Ok(_) => log::info!("query result publish"),
                    Err(_) => log::error!("query result publish error"),
                }

                // send a custom query and wait for the first result
                match dda::publish_query("query_alias", vec![]) {
                    Ok(_) => log::info!("query publish"),
                    Err(_) => log::error!("query publish error"),
                }
            }
            _ => {
                log::error!("wrong dda event type received")
            }
        }
    }

    fn handle_call(_source: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::error!("call should not be used");
        CallRet::NoReply
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
