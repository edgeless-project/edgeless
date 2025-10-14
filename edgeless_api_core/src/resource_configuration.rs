// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#[derive(Clone, PartialEq, Eq)]
pub struct EncodedResourceInstanceSpecification<'a> {
    pub class_type: &'a str,
    pub configuration: heapless::Vec<(&'a str, &'a str), 16>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EncodedPatchRequest<'a> {
    pub instance_id: crate::instance_id::InstanceId,
    pub output_mapping: [Option<(&'a str, crate::instance_id::InstanceId)>; 16],
}

impl<C> minicbor::Encode<C> for EncodedResourceInstanceSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e.str(self.class_type)?;
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
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let id = d.str()?;
        let mut configuration = heapless::Vec::<(&'b str, &'b str), 16>::new();

        for item in d.array_iter::<(&str, &str)>().unwrap().flatten() {
            let _ = configuration.push(item);
        }

        Ok(EncodedResourceInstanceSpecification {
            class_type: id,
            configuration,
        })
    }
}

impl<C> minicbor::CborLen<C> for EncodedResourceInstanceSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let mut len: usize = self.class_type.cbor_len(ctx);

        len += self.configuration[..self.configuration.len()].cbor_len(ctx);

        len
    }
}

impl<C> minicbor::Encode<C> for EncodedPatchRequest<'_> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e.encode(self.instance_id)?;
        {
            let mut true_callbacks_size: u64 = 0;
            for i in self.output_mapping {
                if i.is_some() {
                    true_callbacks_size += 1;
                }
            }
            e = e.array(true_callbacks_size)?;
            for (key, val) in self.output_mapping.into_iter().flatten() {
                e = e.encode((key, val))?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedPatchRequest<'b> {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let id = d.decode::<crate::instance_id::InstanceId>()?;

        let mut outputs: [Option<(&'b str, crate::instance_id::InstanceId)>; 16] = [None; 16];

        for (outputs_i, item) in d
            .array_iter::<(&str, crate::instance_id::InstanceId)>()
            .unwrap()
            .flatten()
            .enumerate()
        {
            outputs[outputs_i] = Some(item);
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

        let mut outputs: [(&str, crate::instance_id::InstanceId); 16] = [(
            "" as &str,
            crate::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
        ); 16];

        let mut num_valid_entries = 0;
        for (outputs_i, (key, val)) in self.output_mapping.into_iter().flatten().enumerate() {
            outputs[outputs_i] = (key, val);
            num_valid_entries += 1;
        }

        len += outputs[..num_valid_entries].cbor_len(ctx);

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

        outputs
            .push((
                "foo",
                crate::instance_id::InstanceId::new(uuid::Uuid::new_v4()),
            ))
            .unwrap();

        let id = super::EncodedResourceInstanceSpecification {
            class_type: "class-1",
            configuration,
        };

        minicbor::encode(id.clone(), &mut buffer[..]).unwrap();

        let len = minicbor::len(id);

        let _id2: super::EncodedResourceInstanceSpecification =
            minicbor::decode(&buffer[..len]).unwrap();

        // assert_eq!(id, id2);
    }
}
