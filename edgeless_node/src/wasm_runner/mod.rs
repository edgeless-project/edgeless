/// Generated host-side of the WASM Component Model.
pub mod wit_binding {
    wasmtime::component::bindgen!({path: "../edgeless_function/wit/edgefunction.wit", async: true});
}

/// Main Implementation of a WASM component function instance.
/// Note that this module only contains the WASM specifics, the generic parts are implemented in the base_runtime.
pub mod function_instance;

/// Bridge between the guest_api_host and the interface defined in the wit binding
pub mod guest_api_binding;

#[cfg(test)]
mod test;
