// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use futures::FutureExt;
use rand::seq::SliceRandom;

/// Each function instance can import a set of functions that need to be implemented on the host-side.
/// This provides the generic host-side implementation of these functions.
/// Those need to be made available to the guest using a virtualization-specific interface/binding.
pub struct GuestAPIHost {
    pub instance_id: edgeless_api::function_instance::InstanceId,
    pub data_plane: edgeless_dataplane::handle::DataplaneHandle,
    pub callback_table: crate::base_runtime::alias_mapping::AliasMapping,
    pub state_handle: Box<dyn crate::state_management::StateHandleAPI>,
    pub telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    pub poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
}

/// Errors to be reported by the host side of the guest binding.
/// This may need to be bridged into the runtime by the virtualization-specific runtime implementation.
#[derive(Debug)]
pub enum GuestAPIError {
    UnknownAlias,
}

impl GuestAPIHost {
    pub async fn cast_alias(&mut self, alias: &str, msg: &str) -> Result<(), GuestAPIError> {
        if alias == "self" {
            self.data_plane.send(self.instance_id.clone(), msg.to_string()).await;
            Ok(())
        } else if let Some(target) = self.callback_table.get_mapping(alias).await {
            match target {
                edgeless_api::common::Output::Single(id) => {
                    self.data_plane.send(id, msg.to_string()).await;
                }
                edgeless_api::common::Output::Any(ids) => {
                    let id = ids.choose(&mut rand::thread_rng());
                    if let Some(id) = id {
                        self.data_plane.send(id.clone(), msg.to_string()).await;
                    } else {
                        return Err(GuestAPIError::UnknownAlias);
                    }
                }
                edgeless_api::common::Output::All(ids) => {
                    for id in ids {
                        self.data_plane.send(id, msg.to_string()).await;
                    }
                }
            }
            Ok(())
        } else {
            Err(GuestAPIError::UnknownAlias)
        }
    }

    pub async fn cast_raw(&mut self, target: edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), GuestAPIError> {
        self.data_plane.send(target, msg.to_string()).await;
        Ok(())
    }

    pub async fn call_alias(&mut self, alias: &str, msg: &str) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        if alias == "self" {
            return self.call_raw(self.instance_id.clone(), msg).await;
            // return Ok(self.data_plane.call(self.instance_id.clone(), msg.to_string()).await);
        } else if let Some(target) = self.callback_table.get_mapping(alias).await {
            // return self.call_raw(target, msg).await;
            match target {
                edgeless_api::common::Output::Single(id) => {
                    // self.data_plane.send(id, msg.to_string()).await;
                    return self.call_raw(id, msg).await;
                }
                edgeless_api::common::Output::Any(ids) => {
                    let id = ids.choose(&mut rand::thread_rng());
                    if let Some(id) = id {
                        // self.data_plane.send(id.clone(), msg.to_string()).await;
                        return self.call_raw(id.clone(), msg).await;
                    } else {
                        return Err(GuestAPIError::UnknownAlias);
                    }
                }
                edgeless_api::common::Output::All(_ids) => {
                    // TODO(raphaelhetzel) introduce new error for this
                    return Err(GuestAPIError::UnknownAlias);
                }
            }
        } else {
            log::warn!("Unknown alias.");
            Err(GuestAPIError::UnknownAlias)
        }
    }

    pub async fn call_raw(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        futures::select! {
            _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                return Ok(edgeless_dataplane::core::CallRet::Err)
            },
            call_res = Box::pin(self.data_plane.call(target, msg.to_string())).fuse() => {
                return Ok(call_res)
            }
        }
    }

    pub async fn telemetry_log(&mut self, lvl: edgeless_telemetry::telemetry_events::TelemetryLogLevel, target: &str, msg: &str) {
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(lvl, target.to_string(), msg.to_string()),
            std::collections::BTreeMap::new(),
        );
    }

    pub async fn slf(&mut self) -> edgeless_api::function_instance::InstanceId {
        self.instance_id.clone()
    }

    pub async fn delayed_cast(&mut self, delay: u64, target_alias: &str, payload: &str) -> Result<(), GuestAPIError> {
        let mut cloned_plane = self.data_plane.clone();
        let cloned_msg = payload.to_string();

        let target = if target_alias == "self" {
            edgeless_api::common::Output::Single(self.instance_id.clone())
        } else if let Some(targted_id) = self.callback_table.get_mapping(target_alias).await {
            targted_id
        } else {
            log::warn!("Unknown alias.");
            return Err(GuestAPIError::UnknownAlias);
        };

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            match target {
                edgeless_api::common::Output::Single(id) => {
                    cloned_plane.send(id, cloned_msg).await;
                }
                edgeless_api::common::Output::Any(ids) => {
                    let id = ids.choose(&mut rand::thread_rng());
                    if let Some(id) = id {
                        cloned_plane.send(id.clone(), cloned_msg).await;
                    } else {
                        log::warn!("Unhandled Situation");
                    }
                }
                edgeless_api::common::Output::All(ids) => {
                    for id in ids {
                        cloned_plane.send(id.clone(), cloned_msg.clone()).await;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn sync(&mut self, serialized_state: &str) -> Result<(), GuestAPIError> {
        self.state_handle.set(serialized_state.to_string()).await;
        log::info!("Function State Sync: {}", serialized_state);
        Ok(())
    }
}
