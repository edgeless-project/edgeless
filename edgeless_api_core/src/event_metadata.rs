// SPDX-FileCopyrightText: © 2023
// SPDX-License-Identifier: MIT

use opentelemetry::trace::{SpanContext, TraceFlags, TraceState};
use opentelemetry::{SpanId, TraceId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventMetadata(SpanContext);

impl EventMetadata {
    pub fn from_uints(trace_id: u128, span_id: u64) -> EventMetadata {
        EventMetadata::from(TraceId::from(trace_id), SpanId::from(span_id))
    }

    pub fn from(trace_id: TraceId, span_id: SpanId) -> EventMetadata {
        EventMetadata(SpanContext::new(trace_id, span_id, TraceFlags::SAMPLED, true, TraceState::NONE))
    }

    pub fn from_bytes(trace_id: [u8; 16], span_id: [u8; 8]) -> EventMetadata {
        EventMetadata::from(TraceId::from_bytes(trace_id), SpanId::from_bytes(span_id))
    }

    pub fn from_event(e: &EventMetadata) -> EventMetadata {
        EventMetadata::from(e.trace_id(), e.span_id())
    }

    pub fn to_bytes(&self) -> [u8; 16 + 8] {
        let mut tmp = [0u8; 24];
        tmp[..16].copy_from_slice(&self.0.trace_id().to_bytes());
        tmp[16..].copy_from_slice(&self.0.span_id().to_bytes());
        tmp
    }

    pub fn trace_id(&self) -> TraceId {
        self.0.trace_id()
    }

    pub fn span_id(&self) -> SpanId {
        self.0.span_id()
    }

    pub fn span_context(&self) -> &SpanContext {
        &self.0
    }

    pub fn empty_new_root() -> Self {
        Self::from(TraceId::INVALID, SpanId::INVALID)
    }

    pub fn empty_dangling_root(offset: u64) -> Self {
        Self::from(TraceId::INVALID, SpanId::from(offset))
    }
}

impl<'b, C> minicbor::Decode<'b, C> for EventMetadata {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        // See https://github.com/twittner/minicbor/blob/develop/minicbor/src/decode.rs#L657
        let octets_16: minicbor::bytes::ByteArray<16> = minicbor::Decode::decode(d, ctx)?;
        let octets_8: minicbor::bytes::ByteArray<8> = minicbor::Decode::decode(d, ctx)?;
        Ok(EventMetadata::from_bytes(octets_16.into(), octets_8.into()))
    }
}

impl<C> minicbor::Encode<C> for EventMetadata {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        // See https://github.com/twittner/minicbor/blob/develop/minicbor/src/encode.rs#L876
        e.bytes(&self.0.trace_id().to_bytes())
            .and_then(|e| e.bytes(&self.0.span_id().to_bytes()))?
            .ok()
    }
}

impl<C> minicbor::CborLen<C> for EventMetadata {
    fn cbor_len(&self, ctx: &mut C) -> usize {
        16.cbor_len(ctx) + 8.cbor_len(ctx) + 16 + 8
    }
}

#[cfg(test)]
mod test {

    use crate::event_metadata::EventMetadata;

    #[test]
    fn test_event_metadata_encoding() {
        let mut buffer = [0u8; 8 + 16 + 2];

        let md = EventMetadata::from_uints(0x42a42bdecaf00001u128, 0x42a42bdecaf00002u64);

        minicbor::encode(&md, &mut buffer[..]).unwrap();

        let len = minicbor::len(&md);

        let decoded: EventMetadata = minicbor::decode(&buffer[..len]).unwrap();

        assert_eq!(md, decoded);
        assert_eq!(buffer.len(), len)
    }

    #[test]
    fn test_event_metadata_bytes() {
        let em_1 = EventMetadata::from_uints(0x42a42bdecaf00003u128, 0x42a42bdecaf00004u64);
        let bytes: [u8; 24] = em_1.to_bytes();
        let x = bytes[..16].try_into();
        let y = bytes[16..].try_into();
        assert!(x.is_ok());
        let x = x.unwrap();
        assert!(y.is_ok());
        let y = y.unwrap();
        let em_2 = EventMetadata::from_bytes(x, y);
        assert_eq!(em_1, em_2)
    }
}
