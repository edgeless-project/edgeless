// SPDX-FileCopyrightText: © 2024 Technical University of Crete
// SPDX-License-Identifier: MIT

use rs_docker::container::DeviceStruct;

pub fn get_sgx_out_of_tree_driver() -> DeviceStruct {
    DeviceStruct {
        CgroupPermissions: "rwm".to_string(),
        PathOnHost: "/dev/isgx".to_string(),
        PathInContainer: "/dev/isgx".to_string(),
    }
}

pub fn get_sgx_in_tree_driver() -> DeviceStruct {
    DeviceStruct {
        CgroupPermissions: "rwm".to_string(),
        PathOnHost: "/dev/sgx_enclave".to_string(),
        PathInContainer: "/dev/sgx_enclave".to_string(),
    }
}

pub fn get_sgx_nuc_driver() -> DeviceStruct {
    get_sgx_out_of_tree_driver()
}
