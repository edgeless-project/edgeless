// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone, PartialEq, Eq)]
pub struct EncodedResourceInstanceSpecification<'a> {
    pub class_type: &'a str,
    pub output_mapping: heapless::Vec<(&'a str, crate::common::Output), 16>,
    pub configuration: heapless::Vec<(&'a str, &'a str), 16>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EncodedPatchRequest<'a> {
    pub instance_id: crate::instance_id::InstanceId,
    pub output_mapping: heapless::Vec<(&'a str, crate::common::Output), 16>,
}

impl<C> minicbor::Encode<C> for EncodedResourceInstanceSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e.str(self.class_type)?;
        {
            e = e.array(self.output_mapping.len() as u64)?;
            for data in &self.output_mapping {
                e = e.encode(data)?;
            }
        }

        {
            e = e.array(self.configuration.len() as u64)?;
            for data in &self.configuration {
                e = e.encode(data)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedResourceInstanceSpecification<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let id = d.str()?;
        let mut outputs = heapless::Vec::<(&'b str, crate::common::Output), 16>::new();
        let mut configuration = heapless::Vec::<(&'b str, &'b str), 16>::new();

        for item in d.array_iter::<(&str, crate::common::Output)>().unwrap() {
            if let Ok(item) = item {
                outputs.push(item);
            }
        }

        for item in d.array_iter::<(&str, &str)>().unwrap() {
            if let Ok(item) = item {
                configuration.push(item);
            }
        }

        Ok(EncodedResourceInstanceSpecification {
            class_type: id,
            output_mapping: outputs,
            configuration,
        })
    }
}

impl<C> minicbor::CborLen<C> for EncodedResourceInstanceSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let mut len: usize = self.class_type.cbor_len(ctx);

        len += self.output_mapping[..self.output_mapping.len()].cbor_len(ctx);

        len += self.configuration[..self.configuration.len()].cbor_len(ctx);

        len
    }
}

impl<C> minicbor::Encode<C> for EncodedPatchRequest<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e.encode(self.instance_id)?;
        {
            e = e.array(self.output_mapping.len() as u64)?;
            for data in &self.output_mapping {
                e = e.encode(data)?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedPatchRequest<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let id = d.decode::<crate::instance_id::InstanceId>()?;

        let mut outputs = heapless::Vec::new();

        for item in d.array_iter::<(&str, crate::common::Output)>().unwrap() {
            if let Ok(item) = item {
                outputs.push(item);
            }
        }

        Ok(EncodedPatchRequest {
            instance_id: id,
            output_mapping: outputs,
        })
    }
}

impl<C> minicbor::CborLen<C> for EncodedPatchRequest<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let mut len: usize = self.instance_id.cbor_len(ctx);

        // let mut outputs: heapless::Vec<(&str, crate::common::Output), 16> = heapless::Vec::new();

        // for item in &self.output_mapping {
        //     outputs.push(item.clone_into(target));
        // }

        len += &self.output_mapping[..].cbor_len(ctx);

        len
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn no_config() {
        let mut buffer = [0 as u8; 1000];

        let mut outputs = heapless::Vec::<(&str, crate::common::Output), 16>::new();
        let configuration = heapless::Vec::<(&str, &str), 16>::new();

        outputs
            .push((
                "foo",
                crate::common::Output::Single(crate::common::Target {
                    instance_id: crate::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
                    port_id: crate::port::Port::<32>(heapless::String::<32>::new()),
                }),
            ))
            .unwrap();

        let id = super::EncodedResourceInstanceSpecification {
            class_type: "class-1",
            output_mapping: outputs,
            configuration: configuration,
        };

        minicbor::encode(id.clone(), &mut buffer[..]).unwrap();

        let len = minicbor::len(id);

        let _id2: super::EncodedResourceInstanceSpecification = minicbor::decode(&buffer[..len]).unwrap();

        // assert_eq!(id, id2);
    }
}
