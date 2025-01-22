// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct WorkSplitterFun;

impl EdgeFunction for WorkSplitterFun {
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("work_splitter: called with '{}'", str_message);

        let tokens: Vec<&str> = str_message.split(",").collect();
        if tokens.len() != 6 {
            log::error!("work_splitter: expected exactly 6 tokens in input string, but got {}", tokens.len());
        } else {
            if tokens[0].parse::<usize>().is_ok() && tokens[1].parse::<usize>().is_ok() {
                if tokens[2].parse::<f64>().is_ok()
                    && tokens[3].parse::<f64>().is_ok()
                    && tokens[4].parse::<f64>().is_ok()
                    && tokens[5].parse::<f64>().is_ok()
                {
                    cast("chunk-metadata", &str_message.as_bytes());
                    cast("chunkinfo", &str_message.as_bytes());
                } else {
                    log::error!("work_splitter: error parsing elements #3 - #6 in input string; one or more is not a float value");
                }
            } else {
                log::error!("work_splitter: first or second element in input string is not an int");
            }
        }
    }

    fn handle_call(_src: InstanceId, _encoded_message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(_payload: Option<&[u8]>, _init_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("work_splitter: started");
    }

    fn handle_stop() {
        log::info!("work_splitter: stopped");
    }
}

edgeless_function::export!(WorkSplitterFun);
