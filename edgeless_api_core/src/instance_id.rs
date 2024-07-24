use minicbor::decode;

// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
// TODO(raphaelhetzel) These should be actual types in the future to allow for type-safety.
pub type NodeId = uuid::Uuid;
pub type ComponentId = uuid::Uuid;

pub const NODE_ID_NONE: uuid::Uuid = uuid::uuid!("00000000-0000-0000-0000-fffe00000000");
pub const FUNCTION_ID_NONE: uuid::Uuid = uuid::uuid!("00000000-0000-0000-0000-fffd00000000");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceId {
    pub node_id: NodeId,
    pub function_id: ComponentId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceIdVec<const N: usize>(pub heapless::Vec<InstanceId, N>);

impl<C> minicbor::Encode<C> for InstanceId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let n_id = *self.node_id.as_bytes();
        let f_id = *self.function_id.as_bytes();

        e.bytes(&n_id)?.bytes(&f_id)?;
        Ok(())
    }
}

impl<C> minicbor::CborLen<C> for InstanceId {
    fn cbor_len(&self, _ctx: &mut C) -> usize {
        34
    }
}

impl<C> minicbor::Decode<'_, C> for InstanceId {
    fn decode<'b>(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        // let data: [[u8; 16];2]  = d.decode::<[[u8;16]; 2]>().unwrap();
        let n_id: [u8; 16] = (*d.bytes()?).try_into().unwrap();
        let f_id: [u8; 16] = (*d.bytes()?).try_into().unwrap();

        Ok(Self {
            node_id: uuid::Uuid::from_bytes(n_id),
            function_id: uuid::Uuid::from_bytes(f_id),
        })
    }
}

impl core::fmt::Display for InstanceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "InstanceId(node_id: {}, function_id: {})", self.node_id, self.function_id)
    }
}

impl InstanceId {
    pub fn new(node_id: uuid::Uuid) -> Self {
        Self {
            node_id,
            function_id: uuid::Uuid::new_v4(),
        }
    }
    pub fn none() -> Self {
        Self {
            node_id: NODE_ID_NONE,
            function_id: FUNCTION_ID_NONE,
        }
    }

    pub fn is_none(&self) -> bool {
        if self.node_id == NODE_ID_NONE && self.function_id == FUNCTION_ID_NONE {
            true
        } else {
            false
        }
    }
}

impl<C, const N: usize> minicbor::Encode<C> for InstanceIdVec<N> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(&self.0[..])?;
        Ok(())
    }
}

impl<C, const N: usize> minicbor::CborLen<C> for InstanceIdVec<N> {
    fn cbor_len(&self, _ctx: &mut C) -> usize {
        minicbor::len(&self.0[..])
    }
}

impl<C, const N: usize> minicbor::Decode<'_, C> for InstanceIdVec<N> {
    fn decode(d: &mut decode::Decoder<'_>, _ctx: &mut C) -> Result<Self, decode::Error> {
        let mut s = Self(heapless::Vec::<InstanceId, N>::new());
        for item in d.array_iter::<InstanceId>().unwrap() {
            if let Ok(item) = item {
                s.0.push(item);
            }
        }
        Ok(s)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn size_matches() {
        let mut buffer = [0 as u8; 1000];

        let id = super::InstanceId::new(uuid::Uuid::new_v4());

        minicbor::encode(id, &mut buffer[..]).unwrap();

        let len = minicbor::len(id);

        let id2: super::InstanceId = minicbor::decode(&buffer[..len]).unwrap();

        assert_eq!(id, id2);
    }
}
