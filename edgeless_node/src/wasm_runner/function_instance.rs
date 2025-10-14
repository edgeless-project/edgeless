// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use wasmtime::AsContextMut;

/// FunctionInstance implementation allowing to execute functions defined as WASM components.
/// Note that this only contains the WASM specific bindings, while the base_runtime provides the generic runtime functionality.
pub struct WASMFunctionInstance {
    edgeless_mem_alloc: wasmtime::TypedFunc<i32, i32>,
    edgeless_mem_free: wasmtime::TypedFunc<(i32, i32), ()>,
    edgeless_mem_clear: wasmtime::TypedFunc<(), ()>,
    edgefunctione_handle_call: wasmtime::TypedFunc<
        (
            i32, // node_id_ptr
            i32, // component_id_ptr
            i32, // payload_ptr
            i32, // payload_len
            i32, // out_ptr_ptr
            i32, // out_len_ptr
        ),
        i32, // Encoded CallRet
    >,
    edgefunctione_handle_cast: wasmtime::TypedFunc<
        (
            i32, // node_id_ptr
            i32, // component_id_ptr
            i32, // payload_ptr
            i32, // payload_len
        ),
        (),
    >,
    edgefunctione_handle_init: wasmtime::TypedFunc<
        (
            i32, // payload_ptr
            i32, // payload_size
            i32, // serialized_state_ptr
            i32, // serialized_state_size
        ),
        (),
    >,
    edgefunctione_handle_stop: wasmtime::TypedFunc<(), ()>,
    memory: wasmtime::Memory,
    store: wasmtime::Store<super::guest_api_binding::GuestAPI>,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for WASMFunctionInstance {
    async fn instantiate(
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.wasm_bulk_memory(true);
        config.wasm_function_references(true);
        let engine = wasmtime::Engine::new(&config)
            .map_err(|_err| crate::base_runtime::FunctionInstanceError::InternalError)?;
        let module = wasmtime::Module::from_binary(&engine, code).map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!(
                "instantiate failed: {}",
                e
            ))
        })?;
        let mut linker = wasmtime::Linker::new(&engine);

        let mut store: wasmtime::Store<super::guest_api_binding::GuestAPI> = wasmtime::Store::new(
            &engine,
            super::guest_api_binding::GuestAPI {
                host: guest_api_host
                    .take()
                    .expect("the impossible happened: no GuestAPIHost"),
            },
        );

        linker
            .func_wrap4_async(
                "env",
                "cast_raw_asm",
                |store,
                 instance_node_id_ptr,
                 instance_component_id_ptr,
                 payload_ptr,
                 payload_len| {
                    Box::new(super::guest_api_binding::cast_raw(
                        store,
                        instance_node_id_ptr,
                        instance_component_id_ptr,
                        payload_ptr,
                        payload_len,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap4_async(
                "env",
                "cast_asm",
                |store, target_ptr, target_len, payload_ptr, payload_len| {
                    Box::new(super::guest_api_binding::cast(
                        store,
                        target_ptr,
                        target_len,
                        payload_ptr,
                        payload_len,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap6_async(
                "env",
                "call_raw_asm",
                |store,
                 instance_node_id_ptr,
                 instance_component_id_ptr,
                 payload_ptr,
                 payload_len,
                 out_ptr_ptr,
                 out_len_ptr| {
                    Box::new(super::guest_api_binding::call_raw(
                        store,
                        instance_node_id_ptr,
                        instance_component_id_ptr,
                        payload_ptr,
                        payload_len,
                        out_ptr_ptr,
                        out_len_ptr,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap6_async(
                "env",
                "call_asm",
                |store,
                 target_ptr,
                 target_len,
                 payload_ptr,
                 payload_len,
                 out_ptr_ptr,
                 out_len_ptr| {
                    Box::new(super::guest_api_binding::call(
                        store,
                        target_ptr,
                        target_len,
                        payload_ptr,
                        payload_len,
                        out_ptr_ptr,
                        out_len_ptr,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap5_async(
                "env",
                "telemetry_log_asm",
                |store, level, target_ptr, target_len, msg_ptr, msg_len| {
                    Box::new(super::guest_api_binding::telemetry_log(
                        store, level, target_ptr, target_len, msg_ptr, msg_len,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap2_async(
                "env",
                "slf_asm",
                |store, out_node_id_ptr, out_component_id_ptr| {
                    Box::new(super::guest_api_binding::slf(
                        store,
                        out_node_id_ptr,
                        out_component_id_ptr,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap5_async(
                "env",
                "delayed_cast_asm",
                |store, delay_ms, target_ptr, target_len, payload_ptr, payload_len| {
                    Box::new(super::guest_api_binding::delayed_cast(
                        store,
                        delay_ms,
                        target_ptr,
                        target_len,
                        payload_ptr,
                        payload_len,
                    ))
                },
            )
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        linker
            .func_wrap2_async("env", "sync_asm", |store, state_ptr, state_len| {
                Box::new(super::guest_api_binding::sync(store, state_ptr, state_len))
            })
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;

        let instance = linker
            .instantiate_async(&mut store, &module)
            .await
            .expect("could not instantiate async linker");

        Ok(Box::new(Self {
            edgeless_mem_alloc: instance
                .get_typed_func::<i32, i32>(&mut store, "edgeless_mem_alloc")
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "edgeless_mem_alloc not available: {}",
                        e
                    ))
                })?,
            edgeless_mem_free: instance
                .get_typed_func::<(i32, i32), ()>(&mut store, "edgeless_mem_free")
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "edgeless_mem_free not available: {}",
                        e
                    ))
                })?,
            edgeless_mem_clear: instance
                .get_typed_func::<(), ()>(&mut store, "edgeless_mem_clear")
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "edgeless_mem_clear not available: {}",
                        e
                    ))
                })?,
            edgefunctione_handle_call: instance
                .get_typed_func::<(i32, i32, i32, i32, i32, i32), i32>(
                    &mut store,
                    "handle_call_asm",
                )
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "handle_call_asm not available: {}",
                        e
                    ))
                })?,
            edgefunctione_handle_cast: instance
                .get_typed_func::<(i32, i32, i32, i32), ()>(&mut store, "handle_cast_asm")
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "handle_cast_asm not available: {}",
                        e
                    ))
                })?,
            edgefunctione_handle_init: instance
                .get_typed_func::<(i32, i32, i32, i32), ()>(&mut store, "handle_init_asm")
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "handle_init_asm not available: {}",
                        e
                    ))
                })?,
            edgefunctione_handle_stop: instance
                .get_typed_func::<(), ()>(&mut store, "handle_stop_asm")
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "handle_stop_asm not available: {}",
                        e
                    ))
                })?,
            memory: instance.get_memory(&mut store, "memory").ok_or_else(|| {
                crate::base_runtime::FunctionInstanceError::BadCode(
                    "memory not available".to_string(),
                )
            })?,
            store,
        }))
    }

    async fn init(
        &mut self,
        init_payload: Option<&str>,
        serialized_state: Option<&str>,
    ) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let (init_payload_ptr, init_payload_len) = match init_payload {
            Some(payload) => {
                let len = payload.len();
                let ptr = super::helpers::copy_to_vm(
                    &mut self.store.as_context_mut(),
                    &self.memory,
                    &self.edgeless_mem_alloc,
                    payload.as_bytes(),
                )
                .await
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "init failed: {}",
                        e
                    ))
                })?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let (serialized_state_ptr, serialized_state_len) = match serialized_state {
            Some(state) => {
                let len = state.len();
                let ptr = super::helpers::copy_to_vm(
                    &mut self.store.as_context_mut(),
                    &self.memory,
                    &self.edgeless_mem_alloc,
                    state.as_bytes(),
                )
                .await
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "init failed: {}",
                        e
                    ))
                })?;
                (ptr, len as i32)
            }
            None => (0i32, 0i32),
        };

        let ret = {
            self.edgefunctione_handle_init
                .call_async(
                    &mut self.store,
                    (
                        init_payload_ptr,
                        init_payload_len,
                        serialized_state_ptr,
                        serialized_state_len,
                    ),
                )
                .await
                .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
            Ok(())
        };

        if init_payload_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (init_payload_ptr, init_payload_len))
                .await
                .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        }

        if serialized_state_len > 0 {
            self.edgeless_mem_free
                .call_async(
                    &mut self.store,
                    (serialized_state_ptr, serialized_state_len),
                )
                .await
                .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        }

        ret
    }

    async fn cast(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        // Depending on the Function, we might employ a basic arena/bump allocator that we must reset at the end of a transaction.
        // This might be a noop if the function defines a working version of `edgeless_mem_free`.
        self.edgeless_mem_clear
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!(
                    "cast failed: mem_clear {}",
                    e
                ))
            })?;

        let component_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .await
        .map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!(
                "cast failed: copy_to_vm1 {}",
                e
            ))
        })?;
        let node_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .await
        .map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!(
                "cast failed: copy_to_vm2 {}",
                e
            ))
        })?;

        let payload_len = msg.len();
        let payload_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            msg.as_bytes(),
        )
        .await
        .map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!(
                "cast failed: copy_to_vm3 {}",
                e
            ))
        })?;

        let ret = {
            self.edgefunctione_handle_cast
                .call_async(
                    &mut self.store,
                    (
                        node_id_ptr,
                        component_id_ptr,
                        payload_ptr,
                        payload_len as i32,
                    ),
                )
                .await
                .map_err(|e| {
                    crate::base_runtime::FunctionInstanceError::BadCode(format!(
                        "cast failed: call_async {}",
                        e
                    ))
                })?;
            Ok(())
        };

        self.edgeless_mem_free
            .call_async(&mut self.store, (component_id_ptr, 16))
            .await
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        self.edgeless_mem_free
            .call_async(&mut self.store, (node_id_ptr, 16))
            .await
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        if payload_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (payload_ptr, payload_len as i32))
                .await
                .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        }
        ret
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        self.edgeless_mem_clear
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
            })?;

        let component_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.function_id.as_bytes(),
        )
        .await
        .map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
        })?;

        let node_id_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            src.node_id.as_bytes(),
        )
        .await
        .map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
        })?;

        let payload_len = msg.len();
        let payload_ptr = super::helpers::copy_to_vm(
            &mut self.store.as_context_mut(),
            &self.memory,
            &self.edgeless_mem_alloc,
            msg.as_bytes(),
        )
        .await
        .map_err(|e| {
            crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
        })?;

        let out_ptr_ptr = self
            .edgeless_mem_alloc
            .call_async(&mut self.store, 4)
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
            })?;

        let out_len_ptr = self
            .edgeless_mem_alloc
            .call_async(&mut self.store, 4)
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
            })?;

        let callret_type = self
            .edgefunctione_handle_call
            .call_async(
                &mut self.store,
                (
                    node_id_ptr,
                    component_id_ptr,
                    payload_ptr,
                    payload_len as i32,
                    out_ptr_ptr,
                    out_len_ptr,
                ),
            )
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!("call failed: {}", e))
            })?;

        let ret = match callret_type {
            0 => Ok(edgeless_dataplane::core::CallRet::NoReply),
            1 => {
                // load the output pointer (inside the WASM memory) (layer of indirection to work around only using one return param)
                let out_ptr: [u8; 4] = self.memory.data_mut(&mut self.store)
                    [out_ptr_ptr as usize..(out_ptr_ptr as usize) + 4]
                    .try_into()
                    .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
                let out_ptr = i32::from_le_bytes(out_ptr);

                // load the output lenght (layer of indirection to work around only using one return param)
                let out_len: [u8; 4] = self.memory.data_mut(&mut self.store)
                    [out_len_ptr as usize..(out_len_ptr as usize) + 4]
                    .try_into()
                    .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
                let out_len = i32::from_le_bytes(out_len);

                // load the atual output param
                let out_raw = self.memory.data_mut(&mut self.store)
                    [out_ptr as usize..(out_ptr as usize) + out_len as usize]
                    .to_vec();
                // TODO(raphaelhetzel) This unwrap can be removed after we migrate the dataplane to use string slices.
                let out = unsafe { std::string::String::from_utf8_unchecked(out_raw) };
                Ok(edgeless_dataplane::core::CallRet::Reply(out))
            }
            _ => Ok(edgeless_dataplane::core::CallRet::Err),
        };

        self.edgeless_mem_free
            .call_async(&mut self.store, (component_id_ptr, 16))
            .await
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        self.edgeless_mem_free
            .call_async(&mut self.store, (node_id_ptr, 16))
            .await
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call_async(&mut self.store, (out_ptr_ptr, 4))
            .await
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        // We don't need to free the data referred to by this pointer as we assume them to be stack-allocated.
        self.edgeless_mem_free
            .call_async(&mut self.store, (out_len_ptr, 4))
            .await
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;

        if payload_len > 0 {
            self.edgeless_mem_free
                .call_async(&mut self.store, (payload_ptr, payload_len as i32))
                .await
                .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
        }

        ret
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        self.edgeless_mem_clear
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!("stop failed: {}", e))
            })?;
        self.edgefunctione_handle_stop
            .call_async(&mut self.store, ())
            .await
            .map_err(|e| {
                crate::base_runtime::FunctionInstanceError::BadCode(format!("stop failed: {}", e))
            })
    }
}
