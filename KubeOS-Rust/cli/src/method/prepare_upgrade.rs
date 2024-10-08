/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use manager::api;
use serde_json::value::{to_raw_value, RawValue};

use crate::method::callable_method::RpcMethod;

pub struct PrepareUpgradeMethod {
    req: api::UpgradeRequest,
}

impl PrepareUpgradeMethod {
    pub fn new(req: api::UpgradeRequest) -> Self {
        PrepareUpgradeMethod { req }
    }

    pub fn set_prepare_upgrade_request(&mut self, req: api::UpgradeRequest) -> &Self {
        self.req = req;
        self
    }
}

impl RpcMethod for PrepareUpgradeMethod {
    type Response = api::Response;
    fn command_name(&self) -> &'static str {
        "prepare_upgrade"
    }
    fn command_params(&self) -> Vec<Box<RawValue>> {
        vec![to_raw_value(&self.req).unwrap()]
    }
}
#[cfg(test)]
mod tests {
    use manager::api::{CertsInfo, UpgradeRequest};

    use super::*;

    #[test]
    fn test_prepare_upgrade_method() {
        let req = UpgradeRequest {
            version: "v1".into(),
            check_sum: "".into(),
            image_type: "".into(),
            container_image: "".into(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        let mut method = PrepareUpgradeMethod::new(req);
        let new_req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "xxx".into(),
            image_type: "xxx".into(),
            container_image: "xxx".into(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        method.set_prepare_upgrade_request(new_req);
        assert_eq!(method.command_name(), "prepare_upgrade");

        let expected_params = "RawValue({\"version\":\"v2\",\"check_sum\":\"xxx\",\"image_type\":\"xxx\",\"container_image\":\"xxx\",\"image_url\":\"\",\"flag_safe\":false,\"mtls\":false,\"certs\":{\"ca_cert\":\"\",\"client_cert\":\"\",\"client_key\":\"\"}})";
        let actual_params = format!("{:?}", method.command_params()[0]);
        assert_eq!(actual_params, expected_params);
    }
}
