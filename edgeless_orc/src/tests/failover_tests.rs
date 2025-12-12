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
    let active_function_node_id = active_instances.get(&lid_2).unwrap().iter().find(|(_node_id, is_active)| *is_active).unwrap().0;
    let standby_replica_node_id = active_instances.get(&lid_2).unwrap().iter().find(|(_node_id, is_active)| !*is_active).unwrap();

    let mut pid_2_active = uuid::Uuid::nil();
    let mut pid_2_standby = uuid::Uuid::nil();
    let mut replicas_started = 0;
    while replicas_started < 2 {
        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            // f2 should not run on the stable node
            assert_ne!(node_id, setup.stable_node_id);
            assert_eq!(spawn_req, spawn_req_rcvd);
            if node_id == active_function_node_id {
                pid_2_active = new_instance_id.function_id;
            } else if node_id == standby_replica_node_id.0 {
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
    // TODO:1 add test for: a node with redundant function fails, it gets created again, no patching needed
    todo!();
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_hot_redundancy_all_die() {
    // TODO:1 add test for: all nodes with redundant functions and active function fail, function gets stopped
    todo!();
}

// TODO:1 chain - all nodes die until there is no unstable node left for the active function
// TODO:1 not enough other nodes to run the redundant function instances (checks how it handles insufficient resources)
// TODO:1 deployment only gets fixed when a completely new node joins! (not enough nodes before)
// TODO:1 nodes that already contain a copy of this function are not considered for redundancy
// TODO:1 test with a node disappearing and reappearing
// TODO:1 node disappears but not graciously
