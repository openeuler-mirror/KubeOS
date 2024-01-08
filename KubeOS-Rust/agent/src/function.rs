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

pub use jsonrpc_core::Result as RpcResult;
use jsonrpc_core::{Error, ErrorCode};
pub use jsonrpc_derive::rpc;
use log::error;

const RPC_OP_ERROR: i64 = -1;

pub struct RpcFunction;

impl RpcFunction {
    pub fn call<F, T>(f: F) -> RpcResult<T>
    where
        F: FnOnce() -> anyhow::Result<T>,
    {
        (f)().map_err(|e| {
            error!("{:?}", e);
            Error { code: ErrorCode::ServerError(RPC_OP_ERROR), message: format!("{:?}", e), data: None }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpcfunction_call() {
        // Define a mock function that returns a result
        fn mock_ok_function() -> anyhow::Result<u32> {
            Ok(42)
        }
        let result: RpcResult<u32> = RpcFunction::call(mock_ok_function);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        fn mock_err_function() -> anyhow::Result<u32> {
            Err(anyhow::anyhow!("error"))
        }
        let result: RpcResult<u32> = RpcFunction::call(mock_err_function);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::ServerError(RPC_OP_ERROR));
    }
}
