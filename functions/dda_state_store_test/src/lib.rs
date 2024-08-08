// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;

struct DDAStateStoreTest;

impl EdgeFunction for DDAStateStoreTest {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        log::info!("state store: handle_cast");

        // match dda::parse(encoded_message).message {
        //     dda::DDA::StateSubscribeSet(key, value) => {
        //         log::info!("setting");
        //         // use the store apis to store the incoming key and value
        //         dda::store_set(key, "hey".to_string());
        //     }
        //     dda::DDA::StateSubscribeDelete(key) => {
        //         log::info!("deleting");
        //         // use the store apis to remove the incoming key and value
        //         dda::store_delete(key)
        //     }
        //     _ => {
        //         log::error!("this should never happen!")
        //     }
        // }
    }

    fn handle_call(_source: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::info!("state store: handle_call");
        // match dda::parse(encoded_message).message {
        //     dda::DDA::StateSubscribeMembershipChange(id, joined) => {
        //         if joined {
        //             log::info!("a new node has joined the cluster{}", id)
        //         } else {
        //             log::info!("a node has left the cluster: {}", id)
        //         }
        //     }
        //     _ => {
        //         log::error!("this should never happen")
        //     }
        // }
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("state store: 'Init' called. Waiting for actions through DDA!");
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAStateStoreTest);
