use edgeless_function::api::*;
use log;

struct MessagingTest;

impl Edgefunction for MessagingTest {
    fn handle_cast(src: Fid, encoded_message: String) {
        match encoded_message.as_str() {
            "test_cast_output" => {
                cast(&src, "cast_output");
            },
            "test_call_output" => {
                let _res = call(&src, "call_output");
            },
            "test_delayed_cast_output" => {
                let _res = delayed_cast(100, &src, "delayed_cast_output");
            },
            "test_cast_alias_output" => {
                cast_alias("test_alias", "cast_alias_output");
            },
            "test_call_alias_output" => {
                let _res = call_alias("test_alias", "call_alias_output");
            },
            _ => {
                log::info!("Unprocessed Message");
            }
        }
    }

    fn handle_call(src: Fid, encoded_message: String) -> CallRet {
        match encoded_message.as_str() {
            "test_err" => {
                CallRet::Err
            },
            "test_ret" => {
                CallRet::Reply("test_reply".to_string())
            },
            _ => {
                CallRet::Noreply
            }  
        }

    }

    fn handle_init(payload: String, _serialized_state: Option<String>) {
        edgeless_function::init_logger();
        log::info!("Messaging Test Init");
    }

    fn handle_stop() {
        log::info!("Messaging Test Stop");
    }
}

edgeless_function::export!(MessagingTest);
