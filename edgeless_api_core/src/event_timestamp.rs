// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EventTimestamp {
    pub secs: i64,
    pub nsecs: u32,
}

impl<C> minicbor::Encode<C> for EventTimestamp {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        let secs = self.secs.to_ne_bytes();
        let nsecs = self.nsecs.to_ne_bytes();

        e.bytes(&secs)?.bytes(&nsecs)?;
        Ok(())
    }
}

impl<C> minicbor::CborLen<C> for EventTimestamp {
    fn cbor_len(&self, _ctx: &mut C) -> usize {
        16
    }
}

impl<C> minicbor::Decode<'_, C> for EventTimestamp {
    fn decode(d: &mut minicbor::Decoder<'_>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let secs: [u8; 8] = (*d.bytes()?).try_into().unwrap();
        let nsecs: [u8; 4] = (*d.bytes()?).try_into().unwrap();

        Ok(Self {
            secs: i64::from_ne_bytes(secs),
            nsecs: u32::from_ne_bytes(nsecs),
        })
    }
}

impl core::fmt::Display for EventTimestamp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.secs, self.nsecs)
    }
}

#[cfg(test)]
mod test {
    use crate::event_timestamp::EventTimestamp;

    #[test]
    fn test_event_timestamp_encoding() {
        let mut buffer = [0_u8; 16];

        let ts = EventTimestamp { secs: 42, nsecs: 999 };

        minicbor::encode(ts, &mut buffer[..]).unwrap();

        let len = minicbor::len(ts);

        let decoded: EventTimestamp = minicbor::decode(&buffer[..len]).unwrap();

        assert_eq!(ts, decoded);
    }
}
