// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
use tokio::sync::Mutex;

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
    pub event_metadata: Arc<Mutex<Option<edgeless_api::function_instance::EventMetadata>>>,
}

/// Errors to be reported by the host side of the guest binding.
/// This may need to be bridged into the runtime by the virtualization-specific runtime implementation.
#[derive(Debug)]
pub enum GuestAPIError {
    UnknownAlias,
    Timeout,
    PoisonPill,
}

static DATAPLANE_TIMEOUT: u64 = 100;

impl GuestAPIHost {
    pub async fn cast_alias(&mut self, alias: &str, msg: &str) -> Result<(), GuestAPIError> {
        let shared_metadata = { self.event_metadata.lock().await.clone() };
        let metadata = shared_metadata.unwrap_or(edgeless_api::function_instance::EventMetadata::empty_dangling_root(0x42a42bdecaf00022u64));
        if alias == "self" {
            self.data_plane.send(self.instance_id, msg.to_string(), &metadata).await;
            Ok(())
        } else if let Some(target) = self.callback_table.get_mapping(alias).await {
            let timeout = async {
                tokio::time::sleep(Duration::from_millis(DATAPLANE_TIMEOUT)).await;
                ()
            };
            // casts can also time out
            tokio::select! {
                _ = self.data_plane.send(target, msg.to_string()) => {
                    Ok(())
                },
                _ = timeout => {
                    log::error!("cast_alias has timed out");
                    Err(GuestAPIError::Timeout)
                }

            }
        } else {
            Err(GuestAPIError::UnknownAlias)
        }
    }

    pub async fn cast_raw(&mut self, target: edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), GuestAPIError> {
        let shared_metadata = { self.event_metadata.lock().await.clone() };
        let metadata = shared_metadata.unwrap_or(edgeless_api::function_instance::EventMetadata::empty_dangling_root(0x42a42bdecaf00023u64));
        self.data_plane.send(target, msg.to_string(), &metadata).await;
        Ok(())
    }

    pub async fn call_alias(&mut self, alias: &str, msg: &str) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        log::info!("call_alias");
        if alias == "self" {
            self.call_raw(self.instance_id, msg).await
        } else if let Some(target) = self.callback_table.get_mapping(alias).await {
            // TODO: change to tokio::select!
            futures::select! {
                res = Box::pin(self.call_raw(target, msg)).fuse() => {
                    log::info!("call_alias: call okay {:?}", res);
                    return res
                },
                e = Box::pin(tokio::time::sleep(Duration::from_millis(DATAPLANE_TIMEOUT))).fuse() => {
                    log::error!("call_alias: timeout elapsed {:?}", e);
                    // TODO: clean up the receiver in the DataplaneHandle in case this gets
                    // dropped due to timeout
                    return Err(GuestAPIError::Timeout)
                }
            }
        } else {
            log::error!("call_alias: Unknown alias. alias={:?}, callback_table={:?}", alias, self.callback_table);
            Err(GuestAPIError::UnknownAlias)
        }
    }

    // NOTE: just does the raw calling, with possibility of a poison pill
    pub async fn call_raw(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        let shared_metadata = { self.event_metadata.lock().await.clone() };
        let metadata = shared_metadata.unwrap_or(edgeless_api::function_instance::EventMetadata::empty_dangling_root(0x42a42bdecaf00024u64));

        futures::select! {
            _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                Ok(edgeless_dataplane::core::CallRet::Err)
            },
            call_res = Box::pin(self.data_plane.call(target, msg.to_string(), &metadata)).fuse() => {
                Ok(call_res)
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
        self.instance_id
    }

    pub async fn delayed_cast(&mut self, delay: u64, target_alias: &str, payload: &str) -> Result<(), GuestAPIError> {
        let mut cloned_plane = self.data_plane.clone();
        let cloned_msg = payload.to_string();

        let shared_metadata = { self.event_metadata.lock().await.clone() };
        let metadata = shared_metadata.unwrap_or(edgeless_api::function_instance::EventMetadata::empty_dangling_root(0x42a42bdecaf00025u64));

        let target_instance_id = if target_alias == "self" {
            self.instance_id
        } else if let Some(targted_id) = self.callback_table.get_mapping(target_alias).await {
            targted_id
        } else {
            log::warn!("Unknown alias. target={:?}, callback_table={:?}", target_alias, self.callback_table);
            return Err(GuestAPIError::UnknownAlias);
        };

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            cloned_plane.send(target_instance_id, cloned_msg, &metadata).await;
        });

        Ok(())
    }

    pub async fn sync(&mut self, serialized_state: &str) -> Result<(), GuestAPIError> {
        self.state_handle.set(serialized_state.to_string()).await;
        log::info!("Function State Sync: {}", serialized_state);
        Ok(())
    }
}
