// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda;
use edgeless_function::*;
use rand::Rng;

struct DDAStoreTest;

// TODO: this is still broken because of the import of Rng
impl EdgeFunction for DDAStoreTest {
    fn handle_cast(_source: InstanceId, encoded_message: &[u8]) {
        // randomly choose one of the store methods and perform it
        let prefix = "key1".to_string(); // to remove all values with key1 prefix
        let num = rand::thread_rng().gen_range(0..100);
        let key = format!("key{}", num);
        let value = b"Hello DDA!".to_vec();
        let method_num = rand::thread_rng().gen_range(0..=7);
        match method_num {
            0 => {
                let _ = dda::store_get(key);
            }
            1 => {
                let _ = dda::store_set(key, value);
            }
            2 => {
                let _ = dda::store_delete(key);
            }
            3 => {
                let _ = dda::store_delete_all();
            }
            4 => {
                let _ = dda::store_delete_prefix(prefix);
            }
            5 => {
                let _ = dda::store_delete_range("key1".to_string(), "key2".to_string());
            }
            6 => {
                let _ = dda::store_scan_prefix(prefix);
            }
            7 => {
                let _ = dda::store_scan_range("key3".to_string(), "key6".to_string());
            }
            _ => {
                // this will never happen
                return;
            }
        }

        // reinvoke self
        delayed_cast(100, "self", b"");
    }

    fn handle_call(_source: InstanceId, encoded_message: &[u8]) -> CallRet {
        log::error!("call should not be used");
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("state store: 'Init' called. Waiting for actions through DDA!");
        // store test performs invokes itself and performs operations on the DDA
        // store periodically
        delayed_cast(100, "self", b"wakeup");
    }

    fn handle_stop() {}
}

edgeless_function::export!(DDAStoreTest);
