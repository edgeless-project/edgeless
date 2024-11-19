// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_function::*;

struct Sqlx_test;

impl EdgeFunction for Sqlx_test {
    fn handle_cast(_src: InstanceId, message: &[u8]) {
        edgeless_function::init_logger();
        println!("sqlx cast");
    }

    fn handle_call(_src: InstanceId, _message: &[u8]) -> CallRet {
        CallRet::NoReply
    }

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        // println!("sqlx init");
        log::info!("sqlx init");
        // cast("database", b"hello sqlx provider");

        // let ret = call("database", b"SELECT id, name,  result FROM workflow WHERE id=999");
        // let ret = call("database", b"INSERT INTO workflow (id, name, result) Values(1001, 'foobar', 9527)");

        // let ret = call("database", b"UPDATE workflow SET name='football'  WHERE id = 1001");
        let ret = call("database", b"DELETE FROM workflow WHERE id=999");

        // println!("sqlx read result: {:?}", response);
        unsafe {
        let (ret, output_params) = match ret {
            CallRet::NoReply => (0, None),
            CallRet::Reply(reply) => (1, Some(reply.consume())),
            CallRet::Err => (2, None),
        };
        // if let (Some((output_ptr, output_len))) = output_params {
        //     *out_ptr_ptr = output_ptr;
        //     *out_len_ptr = output_len
        // }
        
        log::info!("sqlx read result: {:?}", *(output_params.unwrap().0));

        }
        // println!("sqlx read result: {:?}", ret);
    }

    fn handle_stop() {
        // noop
    }
}

edgeless_function::export!(Sqlx_test);
