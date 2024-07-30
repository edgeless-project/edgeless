#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port<const N: usize>(pub heapless::String<N>);

impl<C, const N: usize> minicbor::Encode<C> for Port<N> {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode(&self.0[..])?;
        Ok(())
    }
}

impl<C, const N: usize> minicbor::CborLen<C> for Port<N> {
    fn cbor_len(&self, _ctx: &mut C) -> usize {
        minicbor::len(&self.0[..])
    }
}

impl<C, const N: usize> minicbor::Decode<'_, C> for Port<N> {
    fn decode(d: &mut minicbor::decode::Decoder<'_>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let mut s = Self(heapless::String::<N>::new());
        let data = d.str().unwrap();
        s.0.push_str(data).unwrap();
        // for item in d.array_iter::<InstanceId>().unwrap() {
        //     if let Ok(item) = item {
        //         s.0.push(item);
        //     }
        // }
        Ok(s)
    }
}
