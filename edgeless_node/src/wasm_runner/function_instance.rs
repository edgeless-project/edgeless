// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-License-Identifier: MIT
/// FunctionInstance implementation allowing to execute functions defined as WASM components.
/// Note that this only contains the WASM specific bindings, while the base_runtime provides the generic runtime functionality.
pub struct WASMFunctionInstance {
    store: wasmtime::Store<super::guest_api_binding::GuestAPI>,
    vm_binding: super::wit_binding::Edgefunction,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for WASMFunctionInstance {
    async fn instantiate(
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).map_err(|_err| crate::base_runtime::FunctionInstanceError::InternalError)?;
        let component =
            wasmtime::component::Component::from_binary(&engine, code).map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        super::wit_binding::Edgefunction::add_to_linker(&mut linker, |state: &mut super::guest_api_binding::GuestAPI| state)
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)?;
        let mut store: wasmtime::Store<super::guest_api_binding::GuestAPI> =
            wasmtime::Store::new(&engine, super::guest_api_binding::GuestAPI { api_host: guest_api_host });
        let (binding, _instance) = super::wit_binding::Edgefunction::instantiate_async(&mut store, &component, &linker)
            .await
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)?;
        Ok(Box::new(Self {
            store: store,
            vm_binding: binding,
        }))
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        self.vm_binding
            .call_handle_init(&mut self.store, init_payload.unwrap_or_default(), serialized_state.as_deref())
            .await
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)?;
        Ok(())
    }

    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        self.vm_binding
            .call_handle_cast(
                &mut self.store,
                &super::wit_binding::InstanceId {
                    node: src.node_id.to_string(),
                    function: src.function_id.to_string(),
                },
                &msg,
            )
            .await
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        let res = self
            .vm_binding
            .call_handle_call(
                &mut self.store,
                &super::wit_binding::InstanceId {
                    node: src.node_id.to_string(),
                    function: src.function_id.to_string(),
                },
                &msg,
            )
            .await
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)?;

        Ok(match res {
            super::wit_binding::CallRet::Err => edgeless_dataplane::core::CallRet::Err,
            super::wit_binding::CallRet::Noreply => edgeless_dataplane::core::CallRet::NoReply,
            super::wit_binding::CallRet::Reply(msg) => edgeless_dataplane::core::CallRet::Reply(msg),
        })
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        self.vm_binding
            .call_handle_stop(&mut self.store)
            .await
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::BadCode)
    }
}
