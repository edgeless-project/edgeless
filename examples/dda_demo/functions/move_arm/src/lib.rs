// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_function::api::*;

// Communication with the outside world (also with resources / other components)
// from an edgless function always happens explicitly over the dataplane calls
// call(), cast() - the first parameter identifies the target component. The
// second parameter is the stringified message that is sent to the other
// component.
// Right now it's all hard-coded in the dda resource definition!!!!

// TODO: import macros / library for dda binding - like in http_ingress / egress
// examples; allow to call dda resource directly from the edgeless function

struct MoveArmFun;

impl Edgefunction for MoveArmFun {
    fn handle_cast(_src: InstanceId, encoded_message: String) {}

    fn handle_call(_src: InstanceId, encoded_message: String) -> CallRet {
        // TODO: we would preferably add a new definition to edgeless_function
        // .wti which would allow us to call the dda resource directly and not
        // through the dataplane -> this would be inconsistent with how the
        // other resources work

        // another option: a set of macros like: dda::publish_action, dda:: that under
        // the hood call the dataplane call() or cast() function with the
        // correct parameters -> this is also imperfect, because how would we do
        // streaming of multiple responses back / subscribe to
        // queries/actions/events? -> IDL is in the edgefunction.wit file and
        // it's constrained to simple call and casts, with just one response /
        // no response at all

        // if we were to treat dda as a special resource, that could be called
        // from a function without going through the dataplane + we would use
        // simple gRPC calls to access it, would it actually work from WASM
        // runtime? I would need to test this

        // check: server-side streaming / client-side streaming in WASM -> can I
        // get a stream of responses from the dda resource (scenario: call
        // subscribeEvent on the DDA resource and receive 5 responses and then
        // proceed with WASM function execution)? -> I think it's not possible
        log::info!("MoveArmFun called with {}", encoded_message);
        let res = call(&"dda", &"move_arm");

        if let edgeless_function::api::CallRet::Reply(response) = res {
            log::info!("moved arm over DDA with the following response {}", response);
        }
        CallRet::Noreply
    }

    fn handle_init(_payload: String, serialized_state: Option<String>) {
        // TODO: register events that should trigger this function here using
        // API of dda
    }

    fn handle_stop() {}
}

edgeless_function::export!(MoveArmFun);
