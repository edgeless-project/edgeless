pub mod runner;

pub mod function_instance;

/// This module represents the host-side of the guest APIs.
// TODO(raphaelhetzel) Find a better name for this.
pub mod guest_api;

#[cfg(test)]
mod test;
