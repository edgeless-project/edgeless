// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

/// Main Implementation of a WASM component function instance.
/// Note that this module only contains the WASM specifics, the generic parts are implemented in the base_runtime.
pub mod function_instance;

/// Bridge between the guest_api_host and the interface defined in the wit binding
pub mod guest_api_binding;

pub mod runtime;

mod helpers;

#[cfg(test)]
mod test;
