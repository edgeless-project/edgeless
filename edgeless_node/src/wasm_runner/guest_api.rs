use edgeless_dataplane::core::CallRet;

/// Generated host-side of the WASM Component Model.
pub mod wit_binding {
    wasmtime::component::bindgen!({path: "../edgeless_function/wit/edgefunction.wit", async: true});
}

/// State of a function instance that is accessible from the function itself (via bindings).
/// This struct allows the function to interact with other entities.
pub struct GuestAPI {
    pub instance_id: edgeless_api::function_instance::InstanceId,
    pub data_plane: edgeless_dataplane::handle::DataplaneHandle,
    pub callback_table: std::sync::Arc<tokio::sync::Mutex<super::function_instance::FunctionInstanceCallbackTable>>,
    pub state_handle: Box<dyn crate::state_management::StateHandleAPI>,
    pub telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
}

#[async_trait::async_trait]
impl wit_binding::EdgefunctionImports for GuestAPI {
    async fn cast(&mut self, alias: String, msg: String) -> wasmtime::Result<()> {
        if let Some(target) = self.callback_table.lock().await.alias_map.get(&alias) {
            self.data_plane.send(target.clone(), msg).await;
            Ok(())
        } else {
            log::warn!("Unknown alias.");
            Ok(())
        }
    }

    async fn cast_raw(&mut self, target: wit_binding::InstanceId, msg: String) -> wasmtime::Result<()> {
        let parsed_target = parse_wit_function_id(&target)?;
        self.data_plane.send(parsed_target, msg).await;
        Ok(())
    }

    async fn call_raw(&mut self, target: wit_binding::InstanceId, msg: String) -> wasmtime::Result<wit_binding::CallRet> {
        let parsed_target = parse_wit_function_id(&target)?;
        let res = self.data_plane.call(parsed_target, msg).await;
        Ok(match res {
            CallRet::Reply(msg) => wit_binding::CallRet::Reply(msg),
            CallRet::NoReply => wit_binding::CallRet::Noreply,
            CallRet::Err => wit_binding::CallRet::Err,
        })
    }

    async fn call(&mut self, alias: String, msg: String) -> wasmtime::Result<wit_binding::CallRet> {
        if let Some(target) = self.callback_table.lock().await.alias_map.get(&alias) {
            let res = self.data_plane.call(target.clone(), msg).await;
            Ok(match res {
                CallRet::Reply(msg) => wit_binding::CallRet::Reply(msg),
                CallRet::NoReply => wit_binding::CallRet::Noreply,
                CallRet::Err => wit_binding::CallRet::Err,
            })
        } else {
            log::warn!("Unknown alias.");
            Ok(wit_binding::CallRet::Err)
        }
    }

    async fn telemetry_log(&mut self, lvl: String, target: String, msg: String) -> wasmtime::Result<()> {
        let parsed_level = edgeless_telemetry::telemetry_events::api_to_telemetry(lvl);
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(parsed_level, target, msg),
            std::collections::BTreeMap::new(),
        );
        Ok(())
    }

    async fn slf(&mut self) -> wasmtime::Result<wit_binding::InstanceId> {
        Ok(wit_binding::InstanceId {
            node: self.instance_id.node_id.to_string(),
            function: self.instance_id.function_id.to_string(),
        })
    }

    async fn delayed_cast_raw(&mut self, delay: u64, target: wit_binding::InstanceId, payload: String) -> wasmtime::Result<()> {
        let mut cloned_plane = self.data_plane.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            if let Ok(parsed_target) = parse_wit_function_id(&target) {
                cloned_plane.send(parsed_target, payload).await;
            } else {
                log::error!("Unhandled failure in delayed message.")
            }
        });
        Ok(())
    }

    async fn sync(&mut self, serialized_state: String) -> wasmtime::Result<()> {
        self.state_handle.set(serialized_state.clone()).await;
        log::info!("Function State Sync: {}", serialized_state);
        Ok(())
    }
}

fn parse_wit_function_id(instance_id: &wit_binding::InstanceId) -> anyhow::Result<edgeless_api::function_instance::InstanceId> {
    Ok(edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::parse_str(&instance_id.node)?,
        function_id: uuid::Uuid::parse_str(&instance_id.function)?,
    })
}
