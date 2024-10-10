// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
#[derive(Clone, PartialEq, Eq)]
pub struct EncodedResourceInstanceSpecification<'a> {
    pub class_type: &'a str,
    pub output_mapping: heapless::Vec<(&'a str, crate::instance_id::InstanceId), 16>,
    pub configuration: heapless::Vec<(&'a str, &'a str), 16>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EncodedPatchRequest<'a> {
    pub instance_id: crate::instance_id::InstanceId,
    pub output_mapping: [Option<(&'a str, crate::instance_id::InstanceId)>; 16],
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
        let mut outputs = heapless::Vec::<(&'b str, crate::instance_id::InstanceId), 16>::new();
        let mut configuration = heapless::Vec::<(&'b str, &'b str), 16>::new();

        for item in d.array_iter::<(&str, crate::instance_id::InstanceId)>().unwrap() {
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
            let mut true_callbacks_size: u64 = 0;
            for i in self.output_mapping {
                if i.is_some() {
                    true_callbacks_size += 1;
                }
            }
            e = e.array(true_callbacks_size)?;
            for data in self.output_mapping {
                if let Some((key, val)) = data {
                    e = e.encode((key, val))?;
                }
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedPatchRequest<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let id = d.decode::<crate::instance_id::InstanceId>()?;

        let mut outputs: [Option<(&'b str, crate::instance_id::InstanceId)>; 16] = [None; 16];
        let mut outputs_i: usize = 0;

        for item in d.array_iter::<(&str, crate::instance_id::InstanceId)>().unwrap() {
            if let Ok(item) = item {
                outputs[outputs_i] = Some(item);
                outputs_i += 1;
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

        let mut outputs: [(&str, crate::instance_id::InstanceId); 16] = [("" as &str, crate::instance_id::InstanceId::new(uuid::Uuid::new_v4())); 16];
        let mut outputs_i: usize = 0;

        for item in self.output_mapping {
            if let Some((key, val)) = item {
                outputs[outputs_i] = (key, val);
                outputs_i += 1;
            }
        }

        len += outputs[..outputs_i].cbor_len(ctx);

        len
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn no_config() {
        let mut buffer = [0_u8; 1000];

        let mut outputs = heapless::Vec::<(&str, crate::instance_id::InstanceId), 16>::new();
        let configuration = heapless::Vec::<(&str, &str), 16>::new();

        outputs.push(("foo", crate::instance_id::InstanceId::new(uuid::Uuid::new_v4()))).unwrap();

        let id = super::EncodedResourceInstanceSpecification {
            class_type: "class-1",
            output_mapping: outputs,
            configuration,
        };

        minicbor::encode(id.clone(), &mut buffer[..]).unwrap();

        let len = minicbor::len(id);

        let _id2: super::EncodedResourceInstanceSpecification = minicbor::decode(&buffer[..len]).unwrap();

        // assert_eq!(id, id2);
    }
}
