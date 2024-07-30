// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::container_function::ContainerFunctionAPI;

/// FunctionInstance implementation allowing to execute functions defined
/// as computational containers through a gRPC API.
pub struct ContainerFunctionInstance {
    /// gRPC function client to interact with the container function.
    _function_client: edgeless_api::grpc_impl::container_function::ContainerFunctionAPIClient,
    /// Protocol-neutral API to interact with the container function.
    function_client_api: Box<dyn edgeless_api::guest_api_function::GuestAPIFunction>,
    /// ID of the Docker container created.
    /// Not defined if plain gRPC was used.
    id: Option<String>,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for ContainerFunctionInstance {
    async fn instantiate(
        instance_id: &edgeless_api::function_instance::InstanceId,
        runtime_configuration: std::collections::HashMap<String, String>,
        _guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        let fun_spec = String::from_utf8(code.to_vec()).unwrap_or_default();
        log::info!("container run-time: instantiate {}", fun_spec);

        // Assume the fun_spec is one of (examples):
        // - container:edgeless_function:latest
        // - grpc:http://127.0.0.1:1234
        if let Some((fun_type, fun_addr)) = fun_spec.split_once(':') {
            if fun_type != "grpc" && fun_type != "container" {
                log::error!("container function type not implemented: {}", fun_type);
                return Err(crate::base_runtime::FunctionInstanceError::BadCode);
            }

            let mut grpc_address = fun_addr.to_string();
            let mut id = None;
            if fun_type == "container" {
                let mut docker = match super::docker_utils::Docker::connect() {
                    Ok(docker) => docker,
                    Err(err) => {
                        log::error!("could not connect to Docker: {}", err);
                        return Err(crate::base_runtime::FunctionInstanceError::InternalError);
                    }
                };

                let (fun_id, public_port) = match super::docker_utils::Docker::start(&mut docker, fun_addr.to_string()) {
                    Ok((id, port)) => (id, port),
                    Err(err) => {
                        log::error!("could not create container with image {}: {}", fun_addr, err);
                        return Err(crate::base_runtime::FunctionInstanceError::InternalError);
                    }
                };

                grpc_address = format!("http://127.0.0.1:{}/", public_port);
                log::info!("started container image {} ID {} GuestAPIFunction URL {}", fun_addr, fun_id, grpc_address);
                id = Some(fun_id);
            }

            // TODO(ccicconetti) timeout is hard-coded to 30 seconds, which might
            // not be enough with big containers
            match edgeless_api::grpc_impl::container_function::ContainerFunctionAPIClient::new(&grpc_address, std::time::Duration::from_secs(30))
                .await
            {
                Ok(mut _function_client) => {
                    let mut function_client_api = _function_client.guest_api_function();

                    match runtime_configuration.get("guest_api_host_url") {
                        Some(url) => {
                            let ts = std::time::Instant::now();
                            loop {
                                match function_client_api
                                    .boot(edgeless_api::guest_api_function::BootData {
                                        guest_api_host_endpoint: url.clone(),
                                        instance_id: instance_id.clone(),
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        return Ok(Box::new(Self {
                                            _function_client,
                                            function_client_api,
                                            id,
                                        }))
                                    }
                                    Err(err) => {
                                        if ts.elapsed() >= std::time::Duration::from_secs(30) {
                                            log::error!("could not boot the container function instance: {}", err);
                                            return Err(crate::base_runtime::FunctionInstanceError::InternalError);
                                        } else {
                                            let _ = tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            log::error!("invalid or missing guest_api_host_url");
                            Err(crate::base_runtime::FunctionInstanceError::InternalError)
                        }
                    }
                }
                Err(err) => {
                    log::error!("could not connect to the function instance at {}: {}", grpc_address, err);
                    Err(crate::base_runtime::FunctionInstanceError::InternalError)
                }
            }
        } else {
            log::error!("invalid container function specifier: {}", fun_spec);
            Err(crate::base_runtime::FunctionInstanceError::BadCode)
        }
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!(
            "container run-time: init, payload {}, serialized_state {} bytes",
            init_payload.unwrap_or_default(),
            serialized_state.unwrap_or_default().len()
        );
        self.function_client_api
            .init(edgeless_api::guest_api_function::FunctionInstanceInit {
                init_payload: init_payload.unwrap_or(&"").to_string(),
                serialized_state: serialized_state.unwrap_or(&"").as_bytes().to_vec(),
            })
            .await
            .or(Err(crate::base_runtime::FunctionInstanceError::InternalError))
    }

    async fn cast(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &str,
    ) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: cast, src {}, msg {} bytes", src, msg.len());
        self.function_client_api
            .cast(edgeless_api::guest_api_function::InputEventData {
                src: src.clone(),
                msg: msg.into(),
            })
            .await
            .or(Err(crate::base_runtime::FunctionInstanceError::InternalError))
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        port: &str,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: call, src {}, msg {} bytes", src, msg.len());
        match self
            .function_client_api
            .call(edgeless_api::guest_api_function::InputEventData {
                src: src.clone(),
                msg: msg.into(),
            })
            .await
        {
            Ok(ret) => match ret {
                edgeless_api::guest_api_function::CallReturn::NoRet => Ok(edgeless_dataplane::core::CallRet::NoReply),
                edgeless_api::guest_api_function::CallReturn::Reply(msg) => {
                    Ok(edgeless_dataplane::core::CallRet::Reply(String::from_utf8(msg).unwrap_or_default()))
                }
                edgeless_api::guest_api_function::CallReturn::Err => Ok(edgeless_dataplane::core::CallRet::Err),
            },
            Err(_) => Err(crate::base_runtime::FunctionInstanceError::InternalError),
        }
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: stop");
        if let Err(err) = self.function_client_api.stop().await {
            log::error!("error when stopping container function: {}", err);
            return Err(crate::base_runtime::FunctionInstanceError::InternalError);
        }

        if let Some(id) = &self.id {
            // we have to stop the container that was started in instantiate()
            let mut docker = match super::docker_utils::Docker::connect() {
                Ok(docker) => docker,
                Err(err) => {
                    log::error!("could not connect to Docker: {}", err);
                    return Err(crate::base_runtime::FunctionInstanceError::InternalError);
                }
            };

            if let Err(err) = super::docker_utils::Docker::stop(&mut docker, id.clone()) {
                log::error!("could not stop container with ID {}: {}", id, err);
                return Err(crate::base_runtime::FunctionInstanceError::InternalError);
            };
        }

        Ok(())
    }
}
