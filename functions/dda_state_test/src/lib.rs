// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;

struct DDAStateTest;

impl EdgeFunction for DDAStateTest {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        match dda::parse(encoded_message) {
            dda::DDA::StateSubscribeSet(key, value) => {
                log::info!("state set {:?}:{:?}", key, value);
                let mut new_key = key.clone();
                if new_key.contains("dda") {
                    // prevent state bombing
                    return;
                } else {
                    // in case the key does not contain the dda key, it means it
                    // was sent by an agent
                    new_key.push_str("dda");
                    let _ = dda::state_propose_set(new_key, value);
                }
            }
            dda::DDA::StateSubscribeDelete(key) => {
                log::info!("state delete {:?}", key);
                let mut key_to_delete = key.clone();
                if key_to_delete.contains("dda") {
                    return;
                }
                key_to_delete.push_str("dda");
                let _ = dda::state_propose_delete(key_to_delete);
            }
            dda::DDA::StateSubscribeMembershipChange(id, joined) => {
                if joined {
                    log::info!("a new node has joined the cluster{}", id)
                } else {
                    log::info!("a node has left the cluster: {}", id)
                }
            }
            _ => {
                log::error!("this should never happen")
            }
        };
    }

    fn handle_call(_source: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::error!("call should not be used");
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("dda_state: 'Init' called. Waiting for actions through DDA!");
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAStateTest);
