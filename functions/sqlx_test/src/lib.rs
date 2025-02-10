// SPDX-FileCopyrightText: © 2024 Chen Chen <cc2181@cam.ac.uk>
// SPDX-License-Identifier: MIT

use edgeless_function::*;

struct SqlxTest;

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct WorkflowState {
    id: String,
    metadata: MyState,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct MyState {
    foo: f32,
    bar: String,
}

fn call_wrapper(msg: &str) -> Option<WorkflowState> {
    match call("database", msg.as_bytes()) {
        CallRet::Reply(msg) => {
            let reply = std::str::from_utf8(&msg).unwrap_or("not UTF8");
            log::info!("Got Reply: {}", reply);
            let cur_state: WorkflowState = serde_json::from_str(reply).unwrap_or_default();
            Some(cur_state)
        }
        CallRet::NoReply => {
            log::warn!("Received empty reply from the database");
            None
        }
        CallRet::Err => {
            log::error!("Error when calling the database");
            None
        }
    }
}

impl EdgeFunction for SqlxTest {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        let new_value = core::str::from_utf8(message).unwrap().parse::<f32>().unwrap_or_default();

        // Only update the state if an old one exists.
        if let Some(old_state) = call_wrapper("SELECT id, metadata FROM WorkflowState WHERE id=$1") {
            log::info!("wf_id {}, old value: {}, new value: {}", old_state.id, old_state.metadata.foo, new_value);
            let new_state = MyState {
                foo: new_value,
                bar: String::from("subsequent"),
            };
            call_wrapper(
                format!(
                    "UPDATE WorkflowState SET metadata='{}'  WHERE id = $1",
                    serde_json::to_string(&new_state).unwrap_or_default()
                )
                .as_str(),
            );
        }
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
        log::warn!("Call method not supported");
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("sqlx function init");

        let initial_value = core::str::from_utf8(init_message.unwrap_or_default())
            .unwrap()
            .parse::<f32>()
            .unwrap_or_default();

        let initial_state = MyState {
            foo: initial_value,
            bar: String::from("initial"),
        };

        call_wrapper(
            format!(
                "INSERT INTO WorkflowState (id, metadata) Values($1, '{}')",
                serde_json::to_string(&initial_state).unwrap_or_default()
            )
            .as_str(),
        );
    }

    fn handle_stop() {
        // Try to clean up state when leaving.
        // Note: this operation fails if the database resource instance
        // is terminated before this method is invoked (currently there is
        // no way to enforce a specific sequence when terminating instances
        // in a workflow).
        log::info!("Try to clean database");
        call_wrapper("DELETE FROM WorkflowState WHERE id=$1");
    }
}

edgeless_function::export!(SqlxTest);
