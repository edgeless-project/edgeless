#[derive(Clone, PartialEq, Eq)]
pub struct EncodedResourceInstanceSpecification<'a> {
    pub provider_id: &'a str,
    pub output_mapping: [Option<(&'a str, crate::instance_id::InstanceId)>; 16],
    pub configuration: [Option<(&'a str, &'a str)>; 16],
}

impl<C> minicbor::Encode<C> for EncodedResourceInstanceSpecification<'_> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let mut e = e.str(self.provider_id)?;
        {
            let mut true_callbacks_size: u64 = 0;
            for i in self.output_mapping {
                if i.is_some() {
                    true_callbacks_size = true_callbacks_size + 1;
                }
            }
            e = e.array(true_callbacks_size)?;
            for data in self.output_mapping {
                if let Some((key, val)) = data {
                    e = e.encode((key, val))?;
                }
            }
        }

        {
            let mut true_config_size: u64 = 0;
            for i in self.configuration {
                if i.is_some() {
                    true_config_size = true_config_size + 1;
                }
            }
            e = e.array(true_config_size)?;
            for data in self.configuration {
                if let Some((key, val)) = data {
                    e = e.encode((key, val))?;
                }
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EncodedResourceInstanceSpecification<'b> {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let id = d.str()?;
        let mut outputs: [Option<(&'b str, crate::instance_id::InstanceId)>; 16] = [None; 16];
        let mut outputs_i: usize = 0;
        let mut configuration: [Option<(&'b str, &'b str)>; 16] = [None; 16];
        let mut configuration_i: usize = 0;

        for item in d.array_iter::<(&str, crate::instance_id::InstanceId)>().unwrap() {
            if let Ok(item) = item {
                outputs[outputs_i] = Some(item);
                outputs_i = outputs_i + 1;
            }
        }

        for item in d.array_iter::<(&str, &str)>().unwrap() {
            if let Ok(item) = item {
                configuration[configuration_i] = Some(item);
                configuration_i = configuration_i + 1;
            }
        }

        Ok(EncodedResourceInstanceSpecification {
            provider_id: id,
            output_mapping: outputs,
            configuration: configuration,
        })
    }
}

impl<C> minicbor::CborLen<C> for EncodedResourceInstanceSpecification<'_> {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        let mut len: usize = self.provider_id.cbor_len(ctx);

        let mut outputs: [(&str, crate::instance_id::InstanceId); 16] = [("" as &str, crate::instance_id::InstanceId::new(uuid::Uuid::new_v4())); 16];
        let mut outputs_i: usize = 0;
        let mut configuration: [(&str, &str); 16] = [("" as &str, "" as &str); 16];
        let mut configuration_i: usize = 0;

        for item in self.output_mapping {
            if let Some((key, val)) = item {
                outputs[outputs_i] = (key, val);
                outputs_i = outputs_i + 1;
            }
        }

        for item in self.configuration {
            if let Some((key, val)) = item {
                configuration[configuration_i] = (key, val);
                configuration_i = configuration_i + 1;
            }
        }

        len = len + outputs[..outputs_i].cbor_len(ctx);

        len = len + configuration[..configuration_i].cbor_len(ctx);

        len
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn no_config() {
        let mut buffer = [0 as u8; 1000];

        let mut outputs: [Option<(&str, crate::instance_id::InstanceId)>; 16] = [None; 16];
        let configuration: [Option<(&str, &str)>; 16] = [None; 16];

        outputs[0] = Some(("foo", crate::instance_id::InstanceId::new(uuid::Uuid::new_v4())));

        let id = super::EncodedResourceInstanceSpecification {
            provider_id: "prov-1",
            output_mapping: outputs,
            configuration: configuration,
        };

        minicbor::encode(id.clone(), &mut buffer[..]).unwrap();

        let len = minicbor::len(id);

        let _id2: super::EncodedResourceInstanceSpecification = minicbor::decode(&buffer[..len]).unwrap();

        // assert_eq!(id, id2);
    }
}
