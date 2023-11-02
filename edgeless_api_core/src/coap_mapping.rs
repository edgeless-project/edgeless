use alloc::vec;
use coap_lite::{MessageClass, MessageType, ResponseType};

pub struct COAPEncoder {}

impl COAPEncoder {
    pub fn encode_invocation_event<'a, Endpoint>(
        endpoint: Endpoint,
        event: crate::invocation::Event<&[u8]>,
        token: u8,
        out_buf: &'a mut [u8],
    ) -> ((&'a mut [u8], Endpoint), &'a mut [u8]) {
        let mut req = coap_lite::CoapRequest::<Endpoint>::new();
        req.set_method(coap_lite::RequestType::Post);
        req.set_path("invocation");
        req.message.set_token(vec![token]);
        req.message.header.set_type(MessageType::NonConfirmable);
        let mut buffer = [0 as u8; 1024];

        let new_event: crate::invocation::Event<&minicbor::bytes::ByteSlice> = crate::invocation::Event::<&minicbor::bytes::ByteSlice> {
            target: event.target,
            source: event.source,
            stream_id: event.stream_id,
            data: match event.data {
                crate::invocation::EventData::Cast(val) => crate::invocation::EventData::Cast(val.into()),
                crate::invocation::EventData::Call(val) => crate::invocation::EventData::Call(val.into()),
                crate::invocation::EventData::CallRet(val) => crate::invocation::EventData::CallRet(val.into()),
                crate::invocation::EventData::CallNoRet => crate::invocation::EventData::CallNoRet,
                crate::invocation::EventData::Err => crate::invocation::EventData::Err,
            },
        };

        minicbor::encode(&new_event, &mut buffer[..]).unwrap();

        let len = minicbor::len(&event);
        let data2 = alloc::vec::Vec::<u8>::from(&buffer[..len]);
        req.message.payload = data2;
        let out = req.message.to_bytes().unwrap();
        let (data, tail) = out_buf.split_at_mut(out.len());
        data.clone_from_slice(&out);
        ((data, endpoint), tail)
    }

    pub fn encode_start_resource<'a, Endpoint>(
        endpoint: Endpoint,
        instance: crate::resource_configuration::EncodedResourceInstanceSpecification,
        token: u8,
        out_buf: &'a mut [u8],
    ) -> ((&'a mut [u8], Endpoint), &'a mut [u8]) {
        let mut req = coap_lite::CoapRequest::<Endpoint>::new();
        req.set_method(coap_lite::RequestType::Post);
        req.set_path("resources/start");
        req.message.set_token(vec![token]);
        let mut buffer = [0 as u8; 1024];
        minicbor::encode(&instance, &mut buffer[..]).unwrap();
        let len = minicbor::len(&instance);
        let data2 = alloc::vec::Vec::<u8>::from(&buffer[..len]);
        req.message.payload = data2;
        let out = req.message.to_bytes().unwrap();
        let (data, tail) = out_buf.split_at_mut(out.len());
        data.clone_from_slice(&out);
        ((data, endpoint), tail)
    }

    pub fn encode_stop_resource<'a, Endpoint>(
        endpoint: Endpoint,
        instance_id: crate::instance_id::InstanceId,
        token: u8,
        out_buf: &'a mut [u8],
    ) -> ((&'a mut [u8], Endpoint), &'a mut [u8]) {
        let mut req = coap_lite::CoapRequest::<Endpoint>::new();
        req.set_method(coap_lite::RequestType::Post);
        req.set_path("resources/stop");
        req.message.set_token(vec![token]);
        let mut buffer = [0 as u8; 512];
        minicbor::encode(&instance_id, &mut buffer[..]).unwrap();
        let len = minicbor::len(&instance_id);
        let data2 = alloc::vec::Vec::<u8>::from(&buffer[..len]);
        req.message.payload = data2;
        let out = req.message.to_bytes().unwrap();
        let (data, tail) = out_buf.split_at_mut(out.len());
        data.clone_from_slice(&out);
        ((data, endpoint), tail)
    }

    pub fn encode_response<'a, Endpoint>(
        endpoint: Endpoint,
        data: &[u8],
        token: u8,
        out_buf: &'a mut [u8],
        ok: bool,
    ) -> ((&'a mut [u8], Endpoint), &'a mut [u8]) {
        let mut packet = coap_lite::Packet::new();
        packet.header.set_version(1);
        packet.header.set_type(MessageType::Acknowledgement);
        packet.header.code = match ok {
            true => MessageClass::Response(coap_lite::ResponseType::Content),
            false => MessageClass::Response(coap_lite::ResponseType::BadRequest),
        };
        packet.set_token(vec![token]);
        packet.payload = alloc::vec::Vec::from(data);
        let out = packet.to_bytes().unwrap();
        let (data, tail) = out_buf.split_at_mut(out.len());
        data.clone_from_slice(&out);
        ((data, endpoint), tail)
    }

    pub fn encode_instance_id<'a>(instance_id: crate::instance_id::InstanceId, out_buf: &'a mut [u8]) -> (&'a mut [u8], &'a mut [u8]) {
        let len = minicbor::len(instance_id);
        let (data, tail) = out_buf.split_at_mut(len);
        minicbor::encode(instance_id, &mut data[..]).unwrap();
        (data, tail)
    }

    pub fn encode_error_response<'a>(error: crate::common::ErrorResponse, out_buf: &'a mut [u8]) -> (&'a mut [u8], &'a mut [u8]) {
        let (data, tail) = out_buf.split_at_mut(minicbor::len(error.summary));
        minicbor::encode(error.summary, &mut data[..]).unwrap();
        (data, tail)
    }
}

pub enum CoapMessage<'a> {
    Invocation(crate::invocation::Event<&'a [u8]>),
    ResourceStart(crate::resource_configuration::EncodedResourceInstanceSpecification<'a>),
    ResourceStop(crate::instance_id::InstanceId),
    Response(&'a [u8], bool),
}

pub struct CoapDecoder {}

impl CoapDecoder {
    pub fn decode<'a>(data: &'a [u8]) -> Result<(CoapMessage<'a>, u8), ()> {
        let packet = coap_lite::Packet::from_bytes(data).unwrap();
        match packet.header.code {
            MessageClass::Request(_) => Self::decode_request(data),
            MessageClass::Response(_) => Self::decode_response(data),
            _ => Err(()),
        }
    }

    pub fn decode_request<'a>(data: &'a [u8]) -> Result<(CoapMessage<'a>, u8), ()> {
        let packet = coap_lite::Packet::from_bytes(data).unwrap();

        let path = match packet.get_option(coap_lite::CoapOption::UriPath) {
            Some(options) => {
                let mut vec = alloc::vec::Vec::new();
                for option in options.iter() {
                    if let Ok(seg) = core::str::from_utf8(option) {
                        vec.push(seg);
                    }
                }
                vec.join("/")
            }
            _ => alloc::string::String::new(),
        };

        let body_len = packet.payload.len();
        let body_ref = &data[(data.len() - body_len)..];
        match &path[..] {
            "invocation" => {
                let event: crate::invocation::Event<&minicbor::bytes::ByteSlice> = minicbor::decode(body_ref).unwrap();
                let new_event: crate::invocation::Event<&[u8]> = crate::invocation::Event::<&[u8]> {
                    target: event.target,
                    source: event.source,
                    stream_id: event.stream_id,
                    data: match event.data {
                        crate::invocation::EventData::Cast(val) => crate::invocation::EventData::Cast(val),
                        crate::invocation::EventData::Call(val) => crate::invocation::EventData::Call(val),
                        crate::invocation::EventData::CallRet(val) => crate::invocation::EventData::CallRet(val),
                        crate::invocation::EventData::CallNoRet => crate::invocation::EventData::CallNoRet,
                        crate::invocation::EventData::Err => crate::invocation::EventData::Err,
                    },
                };
                Ok((CoapMessage::Invocation(new_event), packet.get_token()[0]))
            }
            "resources/start" => {
                let resource_instance_spec: crate::resource_configuration::EncodedResourceInstanceSpecification = minicbor::decode(body_ref).unwrap();
                Ok((CoapMessage::ResourceStart(resource_instance_spec), packet.get_token()[0]))
            }
            "resources/stop" => {
                let resource_id: crate::instance_id::InstanceId = minicbor::decode(body_ref).unwrap();
                Ok((CoapMessage::ResourceStop(resource_id), packet.get_token()[0]))
            }
            _ => Err(()),
        }
    }

    pub fn decode_response<'a>(data: &'a [u8]) -> Result<(CoapMessage<'a>, u8), ()> {
        let packet = coap_lite::Packet::from_bytes(data).unwrap();
        let response = coap_lite::CoapResponse { message: packet };
        let body_len = response.message.payload.len();
        let body_ref = &data[(data.len() - body_len)..];

        let return_status_ok = match response.message.header.code {
            MessageClass::Response(response_type) => match response_type {
                ResponseType::Content => true,
                _ => false,
            },
            _ => true,
        };

        Ok((CoapMessage::Response(body_ref, return_status_ok), response.message.get_token()[0]))
    }

    pub fn decode_instance_id(data: &[u8]) -> Result<crate::instance_id::InstanceId, ()> {
        let parsed = minicbor::decode::<crate::instance_id::InstanceId>(data);
        match parsed {
            Ok(id) => Ok(id),
            Err(_) => Err(()),
        }
    }

    pub fn decode_error_response(data: &[u8]) -> Result<crate::instance_id::InstanceId, ()> {
        let parsed = minicbor::decode::<crate::instance_id::InstanceId>(data);
        match parsed {
            Ok(id) => Ok(id),
            Err(_) => Err(()),
        }
    }
}
