// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EdgelessHTTPMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Patch,
}

#[cfg(feature = "hyper")]
pub fn edgeless_method_to_hyper(method: EdgelessHTTPMethod) -> hyper::Method {
    match method {
        EdgelessHTTPMethod::Get => hyper::Method::GET,
        EdgelessHTTPMethod::Head => hyper::Method::HEAD,
        EdgelessHTTPMethod::Post => hyper::Method::POST,
        EdgelessHTTPMethod::Put => hyper::Method::PUT,
        EdgelessHTTPMethod::Delete => hyper::Method::DELETE,
        EdgelessHTTPMethod::Patch => hyper::Method::PATCH,
    }
}

#[cfg(feature = "hyper")]
pub fn hyper_method_to_edgeless(method: &hyper::Method) -> anyhow::Result<EdgelessHTTPMethod> {
    Ok(match *method {
        hyper::Method::GET => EdgelessHTTPMethod::Get,
        hyper::Method::HEAD => EdgelessHTTPMethod::Head,
        hyper::Method::POST => EdgelessHTTPMethod::Post,
        hyper::Method::PUT => EdgelessHTTPMethod::Put,
        hyper::Method::DELETE => EdgelessHTTPMethod::Delete,
        hyper::Method::PATCH => EdgelessHTTPMethod::Patch,
        _ => {
            return Err(anyhow::anyhow!("Unhandled Method"));
        }
    })
}

pub fn string_method_to_edgeless(method: &str) -> anyhow::Result<EdgelessHTTPMethod> {
    Ok(match method {
        "GET" => EdgelessHTTPMethod::Get,
        "HEAD" => EdgelessHTTPMethod::Head,
        "POST" => EdgelessHTTPMethod::Post,
        "PUT" => EdgelessHTTPMethod::Put,
        "DELETE" => EdgelessHTTPMethod::Delete,
        "PATCH" => EdgelessHTTPMethod::Patch,
        _ => {
            return Err(anyhow::anyhow!("Unhandled Method"));
        }
    })
}

pub fn edgeless_method_to_string(method: EdgelessHTTPMethod) -> String {
    match method {
        EdgelessHTTPMethod::Get => "GET".to_string(),
        EdgelessHTTPMethod::Head => "HEAD".to_string(),
        EdgelessHTTPMethod::Post => "POST".to_string(),
        EdgelessHTTPMethod::Put => "PUT".to_string(),
        EdgelessHTTPMethod::Delete => "DELETE".to_string(),
        EdgelessHTTPMethod::Patch => "PATCH".to_string(),
    }
}

pub fn request_to_string(request: &EdgelessHTTPRequest) -> String {
    serde_json::to_string(request).unwrap()
}

pub fn request_from_string(request_str: &str) -> anyhow::Result<EdgelessHTTPRequest> {
    Ok(serde_json::from_str(request_str)?)
}

pub fn response_to_string(response: &EdgelessHTTPResponse) -> String {
    serde_json::to_string(response).unwrap()
}

pub fn response_from_string(response_str: &str) -> anyhow::Result<EdgelessHTTPResponse> {
    Ok(serde_json::from_str(response_str)?)
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum EdgelessHTTPProtocol {
    Unknown,
    HTTP,
    HTTPS,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EdgelessHTTPRequest {
    pub method: EdgelessHTTPMethod,
    pub protocol: EdgelessHTTPProtocol,
    pub host: String,
    pub path: String,
    pub body: Option<Vec<u8>>,
    pub headers: std::collections::HashMap<String, String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EdgelessHTTPResponse {
    pub body: Option<Vec<u8>>,
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
}

impl edgeless_function_core::Deserialize for EdgelessHTTPRequest {
    fn deserialize(raw: &[u8]) -> Self {
        let str_message = core::str::from_utf8(raw).unwrap();
        request_from_string(str_message).unwrap()
    }
}

impl edgeless_function_core::Deserialize for EdgelessHTTPResponse {
    fn deserialize(raw: &[u8]) -> Self {
        let str_message = core::str::from_utf8(raw).unwrap();
        response_from_string(str_message).unwrap()
    }
}

impl edgeless_function_core::Serialize for EdgelessHTTPRequest {
    fn serialize(&self) -> Vec<u8> {
        request_to_string(self).as_bytes().to_vec()
    }
}

impl edgeless_function_core::Serialize for EdgelessHTTPResponse {
    fn serialize(&self) -> Vec<u8> {
        response_to_string(self).as_bytes().to_vec()
    }
}
