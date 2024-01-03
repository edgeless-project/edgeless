use edgeless_dataplane::core::CallRet;

/// Binds the WASM component's imports to the function's GuestAPIHost.
pub struct GuestAPI {
    pub api_host: crate::base_runtime::guest_api::GuestAPIHost,
}

#[async_trait::async_trait]
impl super::wit_binding::EdgefunctionImports for GuestAPI {
    async fn cast_raw(&mut self, target: super::wit_binding::InstanceId, msg: String) -> wasmtime::Result<()> {
        let parsed_target = parse_wit_function_id(&target)?;
        Ok(self.api_host.cast_raw(parsed_target, &msg).await.unwrap_or(()))
    }

    async fn cast(&mut self, name: String, msg: String) -> wasmtime::Result<()> {
        Ok(self.api_host.cast_alias(&name, &msg).await.unwrap_or(()))
    }

    async fn call_raw(&mut self, target: super::wit_binding::InstanceId, msg: String) -> wasmtime::Result<super::wit_binding::CallRet> {
        let parsed_target = parse_wit_function_id(&target)?;

        let res = self.api_host.call_raw(parsed_target, &msg).await.unwrap_or(CallRet::Err);

        Ok(match res {
            CallRet::Reply(msg) => super::wit_binding::CallRet::Reply(msg),
            CallRet::NoReply => super::wit_binding::CallRet::Noreply,
            CallRet::Err => super::wit_binding::CallRet::Err,
        })
    }

    async fn call(&mut self, alias: String, msg: String) -> wasmtime::Result<super::wit_binding::CallRet> {
        let res = self.api_host.call_alias(&alias, &msg).await.unwrap_or(CallRet::Err);
        Ok(match res {
            CallRet::Reply(msg) => super::wit_binding::CallRet::Reply(msg),
            CallRet::NoReply => super::wit_binding::CallRet::Noreply,
            CallRet::Err => super::wit_binding::CallRet::Err,
        })
    }

    async fn telemetry_log(&mut self, lvl: String, target: String, msg: String) -> wasmtime::Result<()> {
        let parsed_level = edgeless_telemetry::telemetry_events::api_to_telemetry(lvl);
        self.api_host.telemetry_log(parsed_level, &target, &msg).await;
        Ok(())
    }

    async fn slf(&mut self) -> wasmtime::Result<super::wit_binding::InstanceId> {
        let own_id = self.api_host.slf().await;
        Ok(super::wit_binding::InstanceId {
            node: own_id.node_id.to_string(),
            function: own_id.function_id.to_string(),
        })
    }

    async fn delayed_cast(&mut self, delay: u64, name: String, payload: String) -> wasmtime::Result<()> {
        self.api_host
            .delayed_cast(delay, &name, &payload)
            .await
            .map_err(|_| anyhow::anyhow!("Delayed Cast Error"))
    }

    async fn sync(&mut self, serialized_state: String) -> wasmtime::Result<()> {
        self.api_host
            .sync(&serialized_state)
            .await
            .map_err(|_| anyhow::anyhow!("Delayed Cast Error"))
    }
}

fn parse_wit_function_id(instance_id: &super::wit_binding::InstanceId) -> anyhow::Result<edgeless_api::function_instance::InstanceId> {
    Ok(edgeless_api::function_instance::InstanceId {
        node_id: uuid::Uuid::parse_str(&instance_id.node)?,
        function_id: uuid::Uuid::parse_str(&instance_id.function)?,
    })
}
