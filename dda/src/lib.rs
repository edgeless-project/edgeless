use edgeless_function::*;
use serde::{Deserialize, Serialize};

type PublicationAlias = String;
type CorrelationId = String;

// NOTE: DDA will ignore the value returned from the function it has called - it is
// the programmer's responsiblity to answer to an action etc.

/// These enums are used to type-safely wrap the data-plane messages to and from
/// the dda resource. There are multiple reasons for not simply using the proto
/// messages as the interchange format: 1. it would be impossible to discern
/// between store set and get, since they receive a String as a parameter 2. it
/// would require that the function specifies dummy values for values of e.g.
/// Event fields that can only be reasonably set by the dda resource
/// All-in-all, this solution here massively simplifies the whole integration.
/// We also only pass fields that are useful to the function
///
/// Translate the DDA methods into typed methods directly accessible from within
/// a function.
///
/// NOTE: additional parameters can be easily specified in these enums, like
/// e.g. timeouts / number of responses to wait for etc -> this makes it a
/// perfect vessel for any needed logic
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum DDA {
    // sent from a function to the outside
    ComPublishEvent(PublicationAlias, Vec<u8>),  // event data
    ComPublishAction(PublicationAlias, Vec<u8>), // action data
    ComPublishQuery(PublicationAlias, Vec<u8>),  // query data

    // TODO: add options for publishing directly to a topic too, not only
    // through publication alias
    ComPublishActionResult(CorrelationId, Vec<u8>), // correlation_id, data
    ComPublishQueryResult(CorrelationId, Vec<u8>),  // correlation_id, data

    // based on the configured subscription, sent from the outside to a function
    // results to calls made from within a function
    // in the future they can be expanded to contain multiple responses
    ComSubscribeEvent(Vec<u8>),          // analogous
    ComSubscribeAction(String, Vec<u8>), // correlation_id, data
    ComSubscribeQuery(String, Vec<u8>),  // correlation_id, data

    ComSubscribeActionResult(Vec<u8>),
    ComSubscribeQueryResult(Vec<u8>),

    // state api ->
    StateSubscribeSet(String, Vec<u8>),           // key, value
    StateSubscribeDelete(String),                 // key
    StateSubscribeMembershipChange(String, bool), // id, joined

    StatePublishSet(String, String),
    StatePublishDelete(String),

    // api for the store -> from function to dda
    StoreGet(String),         // key
    StoreSet(String, String), // key, value
    StoreDelete(String),
    StoreDeleteAll(),
    StoreDeletePrefix(String),
    StoreDeleteRange(String, String), // start, end
    StoreScanPrefix(String),          // prefix for scanning
    StoreScanRange(String, String),   // start, end
}

pub fn parse(encoded_msg: &[u8]) -> DDA {
    let dda = serde_json::from_slice::<DDA>(encoded_msg).expect("should never happen");
    return dda;
}

fn decode(msg: OwnedByteBuff) -> DDA {
    return serde_json::from_slice::<DDA>(msg.to_vec().as_slice()).expect("should never happen");
}

fn encode(msg: DDA) -> Vec<u8> {
    let serialized = serde_json::to_vec(&msg).expect("should never happen");
    return serialized;
}

// DDA Com
// separate methods, because the return values from the dataplane calls have
// different semantics based on the pattern!
pub fn publish_event(pub_alias: &str, data: Vec<u8>) -> Result<(), &'static str> {
    let message = DDA::ComPublishEvent(String::from(pub_alias), data);
    match call("dda", encode(message).as_slice()) {
        CallRet::Err => {
            log::error!("DDA: event could not be published");
            Err("publication did not succeed")
        }
        _ => {
            log::info!("DDA: event published");
            Ok(())
        }
    }
}

// TODO: Result could also contain a custom error enum with more information:
// timeout, connection lost, etc. instead of a generic error string
pub fn publish_action(pub_alias: &str, data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    let msg = DDA::ComPublishAction(String::from(pub_alias), data);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => {
            log::error!("DDA");
            Err("could not publish - dda resource might have crashed")
        }
        CallRet::NoReply => {
            log::error!("no ActionResult within the timeout");
            Err("no response within the timeout")
        }
        CallRet::Reply(reply) => match decode(reply) {
            DDA::ComSubscribeActionResult(res) => Ok(res),
            _ => Err("wrong return type: dda resource sent back the wrong message"),
        },
    }
}

pub fn publish_action_topic(topic: &str, data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    // TODO: directly publishes on a topic without the alias
    todo!("implement");
}

pub fn publish_action_result(correlation_id: String, result_data: Vec<u8>) -> Result<(), &'static str> {
    let msg = DDA::ComPublishActionResult(correlation_id, result_data);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => {
            log::error!("DDA");
            Err("publish action result did not work")
        }
        _ => {
            log::info!("dda action result published");
            Ok(())
        }
    }
}

pub fn publish_query(pub_alias: &str, data: Vec<u8>) {
    todo!("implement")
}

pub fn publish_query_result(correlation_id: String, result_data: Vec<u8>) -> Result<(), &'static str> {
    todo!("implement")
}

pub fn state_propose_set(key: String, value: String) -> Result<(), &'static str> {
    let msg = DDA::StatePublishSet(key, value);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => {
            log::error!("could not propose a new set operation");
            Err("did not work")
        }
        _ => {
            log::info!("proposed a new set");
            Ok(())
        }
    }
}

pub fn state_propose_delete(key: String) -> Result<(), &'static str> {
    let msg = DDA::StatePublishDelete(key);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => {
            log::error!("could not propose a new set operation");
            Err("did not work")
        }
        _ => {
            log::info!("proposed a new set");
            Ok(())
        }
    }
}

pub fn store_get(key: String) {
    let msg = DDA::StoreGet(key);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => {
            log::error!("dda: could not get from the store")
        }
        _ => {
            log::info!("dda: executed get successfully")
        }
    }
}

pub fn store_set(key: String, value: String) {
    let msg = DDA::StoreSet(key, value);
    match call("dda", encode(msg).as_slice()) {
        CallRet::Err => {
            log::error!("dda: could not set the store")
        }
        _ => {
            log::info!("dda: executed set successfully")
        }
    }
}

pub fn store_delete(key: String) {
    todo!("implement")
}

pub fn store_delete_all() {
    todo!("implement")
}

pub fn store_delete_prefix(prefix: String) {
    todo!("implement")
}

pub fn store_delete_range(start: String, end: String) {
    todo!("implement")
}

pub fn store_scan_prefix(prefix: String) {
    todo!("implement")
}

pub fn store_scan_range(start: String, end: String) {
    todo!("implement")
}
