// SPDX-License-Identifier: MIT
use edgeless_function::*;
use edgeless_http::*;

struct HttpStressEndpoint;

//   Static variable, defined at init time. OnecLock<T> is a Rust synchronization primitive that ensures that 
//   the passed struct is initialized only once but can be shared multiple times (see explanation).
// static CONFIGURATION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
// static STATE: std::sync::OnceLock<std::sync::Mutex<ExampleState>> = std::sync::OnceLock::new();

impl EdgeFunction for HttpStressEndpoint {

    // Called at function instance creation time
    fn handle_init(_payload: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        log::info!("Method 'Init' called");

        //   Delay initialization of the function instance (for testing purposes)
        // delayed_cast(5000, "self", "wakeup".as_bytes());

        //   If I had this as annotation '"init-payload": "localhost:10000"'
        // if let Some(payload) = payload {
        //     let payload_str = core::str::from_utf8(payload).unwrap();
        //     assert!(CONFIGURATION.set(payload_str.to_string()).is_ok());
        // }
    }

    // Called at function instance termination time
    fn handle_stop() {
        log::info!("Method 'Stop' called");
    }

    // Called at asynchronous events without return value
    fn handle_cast(_src: InstanceId, encoded_message: &[u8]) {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Method 'Cast' called, MSG: {:?}", str_message);

        //   Buils serialized HTTP request, and sends it (as bytes) to resource http_egress (output name is 'parameters')
        // let res = call(
        //     &"parameters",
        //     &edgeless_http::request_to_string(&edgeless_http::EdgelessHTTPRequest {
        //         method: edgeless_http::EdgelessHTTPMethod::Get,            // Get, Head, Post, Put, Delete or Patch
        //         protocol: edgeless_http::EdgelessHTTPProtocol::HTTP,       // HTTP or HTTPS
        //         host: "api.github.com:443".to_string(),           // CONFIGURATION.get().unwrap().clone(),
        //         path: "".to_string(),                             // "/users/raphaelhetzel/keys".to_string(),
        //         body: None,                                       // Some(encoded_message.to_vec()),
        //         headers: std::collections::HashMap::<String, String>::from([
        //             ("Accept".to_string(), "application/vnd.github+json".to_string()),
        //             ("User-Agent".to_string(), "edgeless".to_string()),
        //         ]),
        //         // headers: std::collections::HashMap::<String, String>::new(),
        //     })
        //     .as_bytes(),
        // );

        //   If previous request returned a value, parse it as HTTP response and log its body
        // if let edgeless_function::CallRet::Reply(response) = res {
        //     let parsed: edgeless_http::EdgelessHTTPResponse = edgeless_http::response_from_string(core::str::from_utf8(&response).unwrap()).unwrap();
        //     log::info!("http_stress_endpoint: {:?}", std::str::from_utf8(&parsed.body.unwrap()));
        // }
    }

    // Called at synchronous events with return value
    fn handle_call(_src: InstanceId, encoded_message: &[u8]) -> CallRet {
        let str_message = core::str::from_utf8(encoded_message).unwrap();
        log::info!("Method 'Call' called, MSG: {:?}", str_message);

        let req: EdgelessHTTPRequest = edgeless_http::request_from_string(str_message).unwrap();

        // Send event to another function (if the request had a body)
        match req.body {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(content) => {
                    log::info!("Sending event with body");
                    cast("parameters", content.as_bytes());    // output name: 'parameters'
                    log::info!("Success!!");
                }
                Err(_) => {
                    log::warn!("WARNING: Body is not valid UTF-8 string. Next function will not be invoked.");
                }
            },
            None => {
                log::info!("Sending event without body");
                cast("parameters", b"");
                log::info!("Success!!");
            }
        }

        //   Prepare a response to the HTTP request
        //   Changes depending on req.body
        // let res_params = if req.path == "/read_number" {
        //     if let Some(body) = req.body {
        //         if let Ok(content) = String::from_utf8(body) {
        //             if let Ok(_) = content.parse::<i32>() {
        //                 cast("parsed_value", content.as_bytes());    // makes the cast here !!!
        //                 (200, None)
        //             } else {
        //                 (400, Some(Vec::<u8>::from("body does not contain an integer")))
        //             }
        //         } else {
        //             (400, Some(Vec::<u8>::from("body is not a string")))
        //         }
        //     } else {
        //         (400, Some(Vec::<u8>::from("empty body")))
        //     }
        // } else {
        //     (404, Some(Vec::<u8>::from("invalid path")))
        // };
        // let http_response = EdgelessHTTPResponse {
        //     status: res_params.0,
        //     body: res_params.1,
        //     headers: std::collections::HashMap::<String, String>::new(),
        // };

        //   Changes depending on req.path
        // let http_response = if req.path == "/hello" {
        //     EdgelessHTTPResponse {
        //         status: 200,
        //         body: Some(Vec::<u8>::from("World")),
        //         headers: std::collections::HashMap::<String, String>::new(),
        //     }
        // } else {
        //     EdgelessHTTPResponse {
        //         status: 404,
        //         body: Some(Vec::<u8>::from("Not Found")),
        //         headers: std::collections::HashMap::<String, String>::new(),
        //     }
        // };

        // edgeless_function::CallRet::NoReply
        edgeless_function::CallRet::Reply(OwnedByteBuff::new_from_slice(
            // edgeless_http::response_to_string(&http_response).as_bytes())
            edgeless_http::response_to_string(&EdgelessHTTPResponse {
                status: 200,
                body: Some(Vec::<u8>::from("Hello from http_stress_endpoint!")),
                headers: std::collections::HashMap::<String, String>::new(),
            }).as_bytes()
        ))
    }
}

edgeless_function::export!(HttpStressEndpoint);
