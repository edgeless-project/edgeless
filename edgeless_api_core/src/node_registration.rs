// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Clone, PartialEq, Eq)]
pub struct NodeId(pub uuid::Uuid);

#[derive(Clone)]
pub struct EncodedNodeRegistration<'a> {
    pub node_id: NodeId,
    pub agent_url: &'a str,
    pub invocation_url: &'a str,
    pub resources: heapless::Vec<ResourceProviderSpecification<'a>, 16>, // 4: node capabilities
}

#[derive(Clone)]
pub struct ResourceProviderSpecification<'a> {
    pub provider_id: &'a str,
    pub class_type: &'a str,
    pub outputs: heapless::Vec<&'a str, 4>,
}

impl<C> minicbor::Encode<C> for NodeId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let n_id = *self.0.as_bytes();
        e.bytes(&n_id)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for NodeId {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let n_id: [u8; 16] = (*d.bytes()?).try_into().unwrap();
        Ok(NodeId(uuid::Uuid::from_bytes(n_id)))
    }
}

impl<C> minicbor::CborLen<C> for NodeId {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let n_id = *self.0.as_bytes();
        n_id.cbor_len(ctx)
    }
}

impl<C> minicbor::Encode<C> for EncodedNodeRegistration<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e;
        e = e.encode(self.node_id.clone())?;
        e = e.encode(self.agent_url)?;
        e = e.encode(self.invocation_url)?;

        {
            e = e.array(self.resources.len().try_into().unwrap())?;
            for spec in &self.resources {
                e = e.encode(spec)?;
            }
        }

        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedNodeRegistration<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let id: NodeId = d.decode()?;
        let agent_url: &str = d.str()?;
        let invocation_url: &str = d.str()?;

        let mut resources: heapless::Vec<ResourceProviderSpecification, 16> = heapless::Vec::new();

        for item in d.array_iter::<ResourceProviderSpecification<'b>>().unwrap() {
            if let Ok(item) = item {
                resources.push(item);
            }
        }

        Ok(EncodedNodeRegistration {
            node_id: id,
            agent_url: &agent_url,
            invocation_url: &invocation_url,
            resources: resources,
        })
    }
}

impl<C> minicbor::CborLen<C> for EncodedNodeRegistration<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let len = self.node_id.cbor_len(ctx) + self.agent_url.cbor_len(ctx) + self.invocation_url.cbor_len(ctx);

        let mut resources: heapless::Vec<ResourceProviderSpecification, 16> = heapless::Vec::new();

        for item in &self.resources {
            resources.push(item.clone());
        }

        len + resources[..resources.len()].cbor_len(ctx)
    }
}

impl<C> minicbor::Encode<C> for ResourceProviderSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e;
        e = e.encode(self.provider_id)?;
        e = e.encode(self.class_type)?;

        {
            e = e.array(self.outputs.len().try_into().unwrap())?;
            for output in &self.outputs {
                e = e.encode(output)?;
            }
        }

        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for ResourceProviderSpecification<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let provider_id: &str = d.decode()?;
        let class_type: &str = d.decode()?;

        let mut outputs = heapless::Vec::new();
        for item in d.array_iter::<&str>().unwrap() {
            if let Ok(item) = item {
                outputs.push(item).unwrap();
            }
        }

        Ok(ResourceProviderSpecification {
            provider_id: &provider_id,
            class_type: &class_type,
            outputs: outputs,
        })
    }
}

impl<C> minicbor::CborLen<C> for ResourceProviderSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let len = self.provider_id.cbor_len(ctx) + self.class_type.cbor_len(ctx);

        let mut data: [&str; 16] = [""; 16];
        let mut data_count = 0;

        for i in &self.outputs {
            data[data_count] = i;
            data_count += 1;
        }

        len + data[..data_count].cbor_len(ctx)
    }
}
