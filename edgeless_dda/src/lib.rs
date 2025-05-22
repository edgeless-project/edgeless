// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_function::*;
use serde::{Deserialize, Serialize};

type PublicationAlias = String;
type CorrelationId = String;

/// DDA resource will ignore the value returned from the function it has called
/// (e.g. the return value from a CALL are not even parsed by DDA resource) - it is
/// the programmer's responsiblity to answer to an action etc. We do not
/// prescribe any timeouts or any other mechanism that would ensure that an
/// action that has been received will trigger a response - this is the
/// responsibility of the programmer.
///
/// These enums are used to type-safely wrap the data-plane messages to and from
/// the dda resource. There are multiple reasons for not simply using the proto
/// messages as the interchange format: 1. it would be impossible to discern
/// between store set and get, since they receive a String as a parameter 2. it
/// would require that the function specifies dummy values for values of e.g.
/// Event fields that can only be reasonably set by the dda resource
/// All-in-all, this solution here massively simplifies the whole integration.
/// We also only pass fields that are useful to the function
///
/// Translates the DDA events into typed methods directly accessible from within
/// a function.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DDA {
    // send a DDA message from a function to the outside over an alias
    ComPublishEvent(PublicationAlias, Vec<u8>),     // event data
    ComPublishAction(PublicationAlias, Vec<u8>),    // action data
    ComPublishQuery(PublicationAlias, Vec<u8>),     // query data
    ComPublishActionResult(CorrelationId, Vec<u8>), // context, correlation_id, data
    ComPublishQueryResult(CorrelationId, Vec<u8>),  // correlation_id, data

    // based on the configured subscription, sent from the outside to a function
    // results to calls made from within a function
    // in the future they can be expanded to contain multiple responses
    ComSubscribeEvent(Vec<u8>),                 // data
    ComSubscribeAction(CorrelationId, Vec<u8>), // context, correlation_id, data
    ComSubscribeQuery(CorrelationId, Vec<u8>),  // correlation_id, data

    // wrappers for results of two-way patterns invoked from within a function
    ComSubscribeActionResult(Vec<u8>),
    ComSubscribeQueryResult(Vec<u8>),

    // DDA State API bindings (currently Raft based, strongly consistent
    // key-value store) - subscription side
    StateSubscribeSet(String, Vec<u8>),           // key, value
    StateSubscribeDelete(String),                 // key
    StateSubscribeMembershipChange(String, bool), // id, joined

    // DDA State API bindings - publication side
    StatePublishSet(String, Vec<u8>),
    StatePublishDelete(String),

    // DDA Store API bindings - publication side
    StoreGet(String),          // key
    StoreSet(String, Vec<u8>), // key, value
    StoreDelete(String),
    StoreDeleteAll(),
    StoreDeletePrefix(String),
    StoreDeleteRange(String, String), // start, end
    StoreScanPrefix(String),          // prefix for scanning
    StoreScanRange(String, String),   // start, end

    // DDA Store API bindings - results side
    StoreGetResult(Vec<u8>),                // data
    StoreScanPrefixResult(String, Vec<u8>), // key, value
    StoreScanRangeResult(String, Vec<u8>),  // key, value
}

pub fn parse(encoded_msg: &[u8]) -> DDA {
    serde_json::from_slice::<DDA>(encoded_msg).expect("this must never happen")
}

fn decode(msg: OwnedByteBuff) -> DDA {
    match serde_json::from_slice::<DDA>(msg.to_vec().as_slice()) {
        Ok(x) => x,
        Err(e) => {
            log::error!("could not encode the DDA struct: {:?}", e);
            panic!("unrecoverable!");
        }
    }
}

fn encode(msg: DDA) -> Vec<u8> {
    match serde_json::to_vec(&msg) {
        Ok(x) => x,
        Err(e) => {
            log::error!("could not encode the DDA struct: {:?}", e);
            [].to_vec()
        }
    }
}

// NOTE: this is WIP
#[allow(dead_code)]
fn call_through_dda<T>(event_name: &str, event: DDA, transform: Option<fn(DDA) -> Result<T, String>>) -> Result<T, String>
where
    T: Default,
{
    log::info!("{:?}: calling through dda", event_name);
    // NOTE: instead of calling through dda, we need an async version of that -
    // otherwise functions would end up blocking the dda resource. Easy
    // solution: pause function execution, until a return value has been made
    // available by the dda resource
    match call("dda", encode(event).as_slice()) {
        CallRet::Err => Err(format!("{:?}: did not work", event_name)),
        CallRet::NoReply => Err(format!("{:?}: TODO timeout", event_name)),
        CallRet::Reply(reply) => match transform {
            Some(f) => f(decode(reply)),
            None => Ok(T::default()),
        },
    }
}

///
/// DDA Com API
///
/// NOTE: a lot of repetition here - can be abstracted with a generic method
/// like above once the async call is available
pub fn publish_event(pub_alias: &str, data: Vec<u8>) -> Result<(), &'static str> {
    let message = DDA::ComPublishEvent(String::from(pub_alias), data);
    log::info!("publish_event on {:?}", pub_alias);
    match call("dda", encode(message).as_slice()) {
        CallRet::Err => Err("publish_event: did not succeed"),
        CallRet::NoReply => Err("publish_event: TODO timeout"),
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn publish_action(pub_alias: &str, data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    let msg = DDA::ComPublishAction(String::from(pub_alias), data);
    log::info!("publish_action on {:?}", pub_alias);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("publish_action: did not succeed"),
        CallRet::NoReply => Err("publish_action: TODO timeout"),
        CallRet::Reply(reply) => match decode(reply) {
            DDA::ComSubscribeActionResult(res) => Ok(res),
            _ => Err("wrong return type: dda resource sent back the wrong message"),
        },
    }
}

pub fn publish_action_result(correlation_id: String, result_data: Vec<u8>) -> Result<(), &'static str> {
    let msg = DDA::ComPublishActionResult(correlation_id, result_data);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("publish_action_result: did not work"),
        CallRet::NoReply => Err("publish_action_result: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn publish_query(pub_alias: &str, data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    let msg = DDA::ComPublishQuery(String::from(pub_alias), data);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("publish_query: did not succeed"),
        CallRet::NoReply => Err("publish_query: TODO timeout"),
        CallRet::Reply(reply) => match decode(reply) {
            DDA::ComSubscribeQueryResult(res) => Ok(res),
            _ => Err("wrong return type: dda resource sent back the wrong message"),
        },
    }
}

pub fn publish_query_result(correlation_id: String, result_data: Vec<u8>) -> Result<(), &'static str> {
    let msg = DDA::ComPublishQueryResult(correlation_id, result_data);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("publish_query_result: did not work"),
        CallRet::NoReply => Err("publish_query_result: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

///
/// DDA State API
///
/// NOTE: propose was split into two methods, unlike original DDA
pub fn state_propose_set(key: String, value: Vec<u8>) -> Result<(), &'static str> {
    let msg = DDA::StatePublishSet(key, value);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("state_propose_set: did not work"),
        CallRet::NoReply => Err("state_propose_set: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn state_propose_delete(key: String) -> Result<(), &'static str> {
    let msg = DDA::StatePublishDelete(key);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("state_propose_delete: did not work"),
        CallRet::NoReply => Err("state_propose_delete: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

///
/// DDA Store API
///
pub fn store_get(key: String) -> Result<Vec<u8>, &'static str> {
    let msg = DDA::StoreGet(key);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_get: did not work"),
        CallRet::NoReply => Err("store_get: TODO timeout"),
        CallRet::Reply(reply) => match decode(reply) {
            DDA::StoreGetResult(data) => Ok(data),
            _ => Err("store_get: wrong result type"),
        },
    }
}

pub fn store_set(key: String, value: Vec<u8>) -> Result<(), &'static str> {
    let msg = DDA::StoreSet(key, value);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_set: did not work"),
        CallRet::NoReply => Err("store_set: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn store_delete(key: String) -> Result<(), &'static str> {
    let msg = DDA::StoreDelete(key);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_delete: did not work"),
        CallRet::NoReply => Err("store_delete: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn store_delete_all() -> Result<(), &'static str> {
    let msg = DDA::StoreDeleteAll();
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_delete_all: did not work"),
        CallRet::NoReply => Err("store_delete_all: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn store_delete_prefix(prefix: String) -> Result<(), &'static str> {
    let msg = DDA::StoreDeletePrefix(prefix);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_delete_prefix: did not work"),
        CallRet::NoReply => Err("store_delete_prefix: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn store_delete_range(start: String, end: String) -> Result<(), &'static str> {
    let msg = DDA::StoreDeleteRange(start, end);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_delete_range: did not work"),
        CallRet::NoReply => Err("store_delete_range: TODO timeout"),
        // empty reply means success
        CallRet::Reply(_) => Ok(()),
    }
}

pub fn store_scan_prefix(prefix: String) -> Result<(String, Vec<u8>), &'static str> {
    let msg = DDA::StoreScanPrefix(prefix);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_scan_prefix: did not work"),
        CallRet::NoReply => Err("store_scan_prefix: TODO timeout"),
        CallRet::Reply(reply) => match decode(reply) {
            DDA::StoreScanPrefixResult(key, data) => Ok((key, data)),
            _ => Err("store_scan_prefix: wrong result type"),
        },
    }
}

pub fn store_scan_range(start: String, end: String) -> Result<(String, Vec<u8>), &'static str> {
    let msg = DDA::StoreScanRange(start, end);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => Err("store_scan_range: did not work"),
        CallRet::NoReply => Err("store_scan_range: TODO timeout"),
        CallRet::Reply(reply) => match decode(reply) {
            DDA::StoreScanRangeResult(key, data) => Ok((key, data)),
            _ => Err("store_scan_range: wrong result type"),
        },
    }
}
