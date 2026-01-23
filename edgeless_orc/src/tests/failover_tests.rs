// SPDX-FileCopyrightText: © 2025 Siemens AG <zalewski.lukasz@siemens.com>
// SPDX-License-Identifier: MIT

#![allow(clippy::all)]

use super::test_utils::*;
use crate::orchestrator::*;
use futures::SinkExt;

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_hot_redundancy_graceful() {
    init_logger();
    let mut setup = setup(10, 0).await;

    // Start this workflow
    //
    // f1 -> f2 -> f3
    //
    // One node is "stable", the others can be disconnected
    //
    // f1 & f3 are forced to be allocated on the stable node
    // f2 is forced to be allocated on a node that will disconnect
    // f2 has redundancy factor 2, should survive the disconnection of one node and seamlessly failover
    //

    // Start f1
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_1 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut pid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        pid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f2
    let mut spawn_req = make_spawn_function_request("f2");
    spawn_req.annotations.insert("label_match_all".to_string(), "unstable".to_string());
    spawn_req.replication_factor = Some(2);
    let lid_2 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    let active_instances = setup.proxy.lock().await.fetch_function_instances_to_nodes();
    let active_function_node_id = active_instances
        .get(&lid_2)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| *is_active)
        .unwrap()
        .0;
    let standby_replica_node_id = active_instances
        .get(&lid_2)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| !*is_active)
        .unwrap();

    let mut pid_2_standby = uuid::Uuid::nil();
    let mut replicas_started = 0;
    while replicas_started < 2 {
        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            // f2 should not run on the stable node
            assert_ne!(node_id, setup.stable_node_id);
            assert_eq!(spawn_req, spawn_req_rcvd);
            if node_id == standby_replica_node_id.0 {
                pid_2_standby = new_instance_id.function_id;
            } else {
                panic!("f2 started on an unexpected node");
            }
        }
        replicas_started += 1;
    }

    // Start f3
    let mut spawn_req = make_spawn_function_request("f3");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_3 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut pid_3 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        pid_3 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Patch f1->f2
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_2,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f2->f3
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_2,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_3,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Make sure there are no pending events around.
    no_function_event(&mut setup.nodes).await;

    // Disconnect the unstable node
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(active_function_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    let mut patch_request_1 = None;
    let mut patch_request_2 = None;
    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd)) => {
                    log::info!("start-function");
                    assert_ne!(node_id, setup.stable_node_id);
                    assert_eq!(node_id, new_instance_id.node_id);
                    assert_eq!("f2", spawn_req_rcvd.spec.id);
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.output_mapping.contains_key("out"));
                    // patch for f1->f2
                    if node_id == setup.stable_node_id {
                        patch_request_1 = Some(patch_request);
                    } else if node_id == standby_replica_node_id.0 {
                        // patch for f2->f3
                        patch_request_2 = Some(patch_request);
                    }
                }
                MockAgentEvent::UpdatePeers(req) => {
                    log::info!("update-peers");
                    match req {
                        edgeless_api::node_management::UpdatePeersRequest::Del(del_node_id) => {
                            assert_eq!(active_function_node_id, del_node_id);
                        }
                        _ => panic!("wrong UpdatePeersRequest message"),
                    }
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&9), num_events.get("update-peers"));
    assert_eq!(Some(&2), num_events.get("patch-function"));
    assert_eq!(Some(&1), num_events.get("start-function"));

    let patch_request_1 = patch_request_1.unwrap();
    let patch_request_2 = patch_request_2.unwrap();
    // check if the orchestrator patched f2's outputs to point to the new instance that it failovered to
    assert_eq!(pid_1, patch_request_1.function_id);
    assert_eq!(pid_2_standby, patch_request_1.output_mapping.get("out").unwrap().function_id);
    assert_eq!(pid_2_standby, patch_request_2.function_id);
    assert_eq!(pid_3, patch_request_2.output_mapping.get("out").unwrap().function_id);

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_hot_redundancy_replica_dies() {
    // Test: a node with redundant function (hot-standby) fails, it gets created again on another node, no patching needed
    // 3 nodes, one function, replication_factor == 2
    // ensure that when the node where the hot-standby replica is running dies, a new hot-standby replica gets started on the remaining node, but no patching runs
    init_logger();

    // Setup with 3 nodes (1 stable + 2 unstable)
    let mut setup = setup(3, 0).await;

    // Start a function with replication_factor = 2 on any two nodes (one will be active, one will be hot-standby)
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req.replication_factor = Some(2);
    let lid = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    // Figure out which node has the active instance and which has the standby
    let active_instances = setup.proxy.lock().await.fetch_function_instances_to_nodes();
    let active_function_node_id = active_instances
        .get(&lid)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| *is_active)
        .unwrap()
        .0;
    let standby_replica_node_id = active_instances
        .get(&lid)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| !*is_active)
        .unwrap()
        .0;

    // Wait for both replicas to start
    let mut replicas_started = 0;
    while replicas_started < 2 {
        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(spawn_req, spawn_req_rcvd);
            assert!(node_id == active_function_node_id || node_id == standby_replica_node_id);
            assert_eq!(node_id, new_instance_id.node_id);
        }
        replicas_started += 1;
    }

    // Make sure there are no pending events
    no_function_event(&mut setup.nodes).await;

    // Disconnect the node with the STANDBY replica (not the active one)
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(standby_replica_node_id)).await;

    // Track events - we expect:
    // - update-peers on remaining nodes (2 nodes)
    // - start-function on a node (new standby replica) - can be any feasible node
    // - NO patch-function events (since the active function is still alive, so no patching is needed)
    let mut num_events = std::collections::HashMap::new();
    let mut new_standby_node_id = uuid::Uuid::nil();

    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd)) => {
                    log::info!("start-function on node {}", node_id);
                    // The new standby can be started on any feasible node (random selection)
                    // It should NOT be on the node that was disconnected
                    assert_ne!(node_id, standby_replica_node_id);
                    assert_eq!("f1", spawn_req_rcvd.spec.id);
                    new_standby_node_id = new_instance_id.node_id;
                }
                MockAgentEvent::PatchFunction(_patch_request) => {
                    panic!("No patching should occur when only the standby replica dies");
                }
                MockAgentEvent::UpdatePeers(req) => {
                    log::info!("update-peers on node {}", node_id);
                    match req {
                        edgeless_api::node_management::UpdatePeersRequest::Del(del_node_id) => {
                            assert_eq!(standby_replica_node_id, del_node_id);
                        }
                        _ => panic!("wrong UpdatePeersRequest message"),
                    }
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }

    // Verify expected events:
    // - 2 update-peers (one for each remaining node)
    // - 1 start-function (new standby replica)
    // - 0 patch-function (no patching needed)
    assert_eq!(Some(&2), num_events.get("update-peers"));
    assert_eq!(Some(&1), num_events.get("start-function"));
    assert_eq!(None, num_events.get("patch-function"));

    // Verify the new standby was created on a valid node (not the disconnected one)
    assert_ne!(standby_replica_node_id, new_standby_node_id);

    // The active function should still be on its original node
    let active_instances = setup.proxy.lock().await.fetch_function_instances_to_nodes();
    let current_active_node_id = active_instances
        .get(&lid)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| *is_active)
        .unwrap()
        .0;
    assert_eq!(active_function_node_id, current_active_node_id);

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_hot_redundancy_all_die() {
    // Test: all nodes with redundant functions (both active and hot-standby) fail
    // The workflow should be stopped since KPI-13 cannot be guaranteed
    init_logger();

    // Setup with 3 nodes: 1 stable + 2 unstable
    // We'll put the replicated function on the 2 unstable nodes, then kill both
    let mut setup = setup(3, 0).await;

    // Start a function with replication_factor = 2, forced onto unstable nodes
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req.annotations.insert("label_match_all".to_string(), "unstable".to_string());
    spawn_req.replication_factor = Some(2);
    let lid = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    // Figure out which nodes have the active and standby instances
    let active_instances = setup.proxy.lock().await.fetch_function_instances_to_nodes();
    let active_function_node_id = active_instances
        .get(&lid)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| *is_active)
        .unwrap()
        .0;
    let standby_replica_node_id = active_instances
        .get(&lid)
        .unwrap()
        .iter()
        .find(|(_node_id, is_active)| !*is_active)
        .unwrap()
        .0;

    // Wait for both replicas to start
    let mut replicas_started = 0;
    while replicas_started < 2 {
        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            // Function should only run on unstable nodes
            assert_ne!(node_id, setup.stable_node_id);
            assert_eq!(spawn_req, spawn_req_rcvd);
            assert_eq!(node_id, new_instance_id.node_id);
        }
        replicas_started += 1;
    }

    // Make sure there are no pending events
    no_function_event(&mut setup.nodes).await;

    // Kill BOTH nodes - first the active, then the standby
    // This simulates a catastrophic failure where no hot-standby is available
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(active_function_node_id)).await;
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(standby_replica_node_id)).await;

    // Track events - we expect:
    // - update-peers on remaining node (stable node)
    // - stop-function for the workflow (KPI-13 failure)
    // - possibly some patches as the system tries to recover
    let mut num_events = std::collections::HashMap::new();
    let mut stopped_functions = Vec::new();

    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StartFunction((_, spawn_req_rcvd)) => {
                    log::info!("start-function on node {} for {}", node_id, spawn_req_rcvd.spec.id);
                }
                MockAgentEvent::StopFunction(instance_id) => {
                    log::info!("stop-function on node {} for pid {}", node_id, instance_id.function_id);
                    stopped_functions.push(instance_id);
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function on node {} for pid {}", node_id, patch_request.function_id);
                }
                MockAgentEvent::UpdatePeers(req) => {
                    log::info!("update-peers on node {}", node_id);
                    match req {
                        edgeless_api::node_management::UpdatePeersRequest::Del(del_node_id) => {
                            assert!(del_node_id == active_function_node_id || del_node_id == standby_replica_node_id);
                        }
                        _ => panic!("wrong UpdatePeersRequest message"),
                    }
                }
                _ => {
                    log::info!("other event: {}", event_to_string(&event));
                }
            };
        } else {
            break;
        }
    }

    // Verify: update-peers should have been called
    // First deletion: 2 nodes notified (stable + standby)
    // Second deletion: 1 node notified (stable only)
    // Total: 3 update-peers events
    assert_eq!(Some(&3), num_events.get("update-peers"));

    // The function should no longer be active (it was stopped or has no instances)
    let active_instances = setup.proxy.lock().await.fetch_function_instances_to_nodes();

    // Either the function was completely removed, or it has no instances left
    if let Some(instances) = active_instances.get(&lid) {
        assert!(instances.is_empty(), "Function should have no instances after all nodes died");
    }
    // If the key doesn't exist, thats also correct (function was fully stopped)

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_hot_redundancy_not_enough_nodes() {
    // Test: not enough nodes to run the redundant function instances
    // The workflow spawn request should fail when replication_factor > available nodes
    init_logger();

    // Setup with only 1 node
    let mut setup = setup(1, 0).await;

    // Try to start a function with replication_factor = 2 (requires 2 nodes, but we only have 1)
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req.replication_factor = Some(2);

    let result = setup.fun_client.start(spawn_req.clone()).await.unwrap();

    // The spawn request should fail because there aren't enough nodes
    match result {
        edgeless_api::common::StartComponentResponse::InstanceId(_) => {
            panic!("Expected spawn to fail due to insufficient nodes, but it succeeded");
        }
        edgeless_api::common::StartComponentResponse::ResponseError(err) => {
            log::info!(
                "Spawn correctly failed with error: {} - {}",
                err.summary,
                err.detail.as_deref().unwrap_or("no detail")
            );
            // The error should indicate that no suitable nodes were found or insufficient resources
            assert!(
                err.summary.to_lowercase().contains("fail")
                    || err.detail.as_deref().unwrap_or("").to_lowercase().contains("no suitable")
                    || err.detail.as_deref().unwrap_or("").to_lowercase().contains("no node"),
                "Error message should indicate insufficient nodes: {} - {:?}",
                err.summary,
                err.detail
            );
        }
    };

    // No function should have been started (or if one was started, it should have been cleaned up)
    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_hot_redundancy_not_enough_matching_nodes() {
    // Test: enough nodes exist, but not enough match the constraints for redundancy
    // The workflow spawn request should fail when replication_factor > matching nodes
    init_logger();

    // Setup with 3 nodes (1 stable + 2 unstable)
    let mut setup = setup(3, 0).await;

    // Try to start a function with replication_factor = 2, but constrained to only the stable node
    // Since there's only 1 stable node and we need 2 replicas, this should fail
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req.annotations.insert("label_match_all".to_string(), "stable".to_string());
    spawn_req.replication_factor = Some(2);

    let result = setup.fun_client.start(spawn_req.clone()).await.unwrap();

    // The spawn request should fail because there aren't enough matching nodes
    match result {
        edgeless_api::common::StartComponentResponse::InstanceId(_) => {
            panic!("Expected spawn to fail due to insufficient matching nodes, but it succeeded");
        }
        edgeless_api::common::StartComponentResponse::ResponseError(err) => {
            log::info!(
                "Spawn correctly failed with error: {} - {}",
                err.summary,
                err.detail.as_deref().unwrap_or("no detail")
            );
            // The error should indicate that no suitable nodes were found
            assert!(
                err.summary.to_lowercase().contains("fail")
                    || err.detail.as_deref().unwrap_or("").to_lowercase().contains("no suitable")
                    || err.detail.as_deref().unwrap_or("").to_lowercase().contains("no node"),
                "Error message should indicate insufficient matching nodes: {} - {:?}",
                err.summary,
                err.detail
            );
        }
    };

    // No function should have been started (or if one was started, it should have been cleaned up)
    no_function_event(&mut setup.nodes).await;
}
