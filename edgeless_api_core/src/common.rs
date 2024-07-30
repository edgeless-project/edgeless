// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone, Debug)]
pub struct ErrorResponse {
    pub summary: &'static str,
    pub detail: Option<&'static str>,
}

#[derive(Debug, Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen, PartialEq, Eq)]
pub enum Output {
    #[n(0)]
    Single(#[n(0)] Target),
    #[n(1)]
    Any(#[n(0)] TargetVec<4>),
    #[n(2)]
    All(#[n(0)] TargetVec<4>),
}

#[derive(Debug, Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen, PartialEq, Eq)]
pub struct Target {
    #[n(0)]
    pub instance_id: crate::instance_id::InstanceId,
    #[n(1)]
    pub port_id: crate::port::Port<32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetVec<const N: usize>(pub heapless::Vec<Target, N>);

impl<C, const N: usize> minicbor::Encode<C> for TargetVec<N> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(&self.0[..])?;
        Ok(())
    }
}

impl<C, const N: usize> minicbor::CborLen<C> for TargetVec<N> {
    fn cbor_len(&self, _ctx: &mut C) -> usize {
        minicbor::len(&self.0[..])
    }
}

impl<C, const N: usize> minicbor::Decode<'_, C> for TargetVec<N> {
    fn decode(d: &mut minicbor::decode::Decoder<'_>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut s = Self(heapless::Vec::<Target, N>::new());
        for item in d.array_iter::<Target>().unwrap() {
            if let Ok(item) = item {
                s.0.push(item);
            }
        }
        Ok(s)
    }
}
