// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen)]
pub enum EventData<T> {
    #[n(0)]
    Call(#[n(0)] T),
    #[n(1)]
    Cast(#[n(0)] T),
    #[n(2)]
    CallRet(#[n(0)] T),
    #[n(3)]
    CallNoRet,
    #[n(4)]
    Err,
}

#[derive(Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen)]
pub struct Event<T> {
    #[n(0)]
    pub target: crate::instance_id::InstanceId,
    #[n(1)]
    pub source: crate::instance_id::InstanceId,
    #[n(2)]
    pub stream_id: u64,
    #[n(3)]
    pub data: EventData<T>,
    #[n(4)]
    pub target_port: super::port::Port<32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LinkProcessingResult {
    FINAL,
    PROCESSED,
    PASSED,
}
