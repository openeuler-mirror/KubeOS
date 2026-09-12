/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2025. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{
    fs::DirBuilder,
    os::unix::{fs::DirBuilderExt, io::RawFd},
    path::Path,
    process::exit,
};

use clap::{Parser, Subcommand};
use cli::{
    client::Client,
    method::{callable_method::RpcMethod, prepare::PrepareCmdMethod},
};
use manager::api::{CmdRequest, CertsInfo, ConfigItem};
use nix::{
    fcntl::{flock, open, FlockArg, OFlag},
    libc::{c_int, raise, signal, SIG_DFL, STDERR_FILENO},
    sys::{
        signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal},
        stat::Mode,
    },
    unistd::{close, write},
};

const SOCK_PATH: &str = "/run/os-agent/os-agent.sock";
/// Single-instance lock: only one kbosctl process may run at a time.
/// The lock is held for the whole process lifetime (until the RPC response
/// arrives and the process exits), covering the entire operation window.
const LOCK_PATH: &str = "/run/os-agent/kbosctl.lock";
/// Cancel flag: written by kbosctl on interruption so os-agent aborts the
/// running operation at the next checkpoint.
const CANCEL_PATH: &str = "/run/os-agent/cancel";
/// Interrupt signals that kbosctl can catch and use to notify os-agent.
/// SIGKILL/SIGSTOP cannot be caught: os-agent detects those via the released
/// single-instance lock instead.
const CANCEL_SIGNALS: &[Signal] = &[Signal::SIGINT, Signal::SIGTERM, Signal::SIGHUP, Signal::SIGQUIT];

const CANCEL_MSG: &[u8] = b"kbosctl interrupted: asking os-agent to cancel the operation...\n";

#[derive(Parser)]
#[clap(name = "kbosctl")]
#[clap(about = "CLI tool for KubeOS upgrade and rollback")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Upgrade {
        #[clap(long)]
        os_image: String,
        /// Inject config file: <SRC> <DST>, can be specified multiple times
        #[clap(long = "config", num_args = 2)]
        configs: Vec<String>,
        #[clap(long)]
        skip_tls: bool,
        #[clap(long)]
        reboot: bool,
    },
    Rollback {
        #[clap(long)]
        reboot: bool,
    },
}

/// Acquires an exclusive, non-blocking flock on the given lock file.
/// The returned fd keeps the lock held and must stay alive until the
/// operation completes (it is released when the fd is closed or the
/// process exits, which also covers Ctrl+C and SIGKILL).
fn acquire_lock(lock_path: &str) -> Result<RawFd, String> {
    let lock = Path::new(lock_path);
    if let Some(dir) = lock.parent() {
        if !dir.exists() {
            DirBuilder::new()
                .recursive(true)
                .mode(0o750)
                .create(dir)
                .map_err(|e| format!("Failed to create lock directory {}: {}", dir.display(), e))?;
        }
    }
    let fd = open(lock_path, OFlag::O_CREAT | OFlag::O_RDWR | OFlag::O_CLOEXEC, Mode::from_bits_truncate(0o644))
        .map_err(|e| format!("Failed to open lock file {}: {}", lock_path, e))?;
    match flock(fd, FlockArg::LockExclusiveNonblock) {
        Ok(()) => Ok(fd),
        Err(_) => {
            let _ = close(fd);
            Err(format!(
                "Another kbosctl operation is already in progress, please wait for it to complete (lock: {})",
                lock_path
            ))
        },
    }
}

/// Signal handler for the interrupt signals listed in CANCEL_SIGNALS: write
/// the cancel flag for os-agent, then restore the default disposition and
/// re-raise the received signal so the process terminates with it (releasing
/// the single-instance lock). Only async-signal-safe syscalls are used inside
/// the handler.
extern "C" fn handle_sigint(sig: c_int) {
    // nix signal handling requires unsafe syscalls; the block is kept minimal
    // and only contains async-signal-safe operations.
    #[allow(unsafe_code)]
    unsafe {
        let _ = write(STDERR_FILENO, CANCEL_MSG);
        if let Ok(fd) =
            open(CANCEL_PATH, OFlag::O_CREAT | OFlag::O_WRONLY | OFlag::O_TRUNC, Mode::from_bits_truncate(0o644))
        {
            let _ = write(fd, b"cancel");
            let _ = close(fd);
        }
        signal(sig, SIG_DFL);
        raise(sig);
    }
}

fn main() {
    let cli = Cli::parse();

    // Reject concurrent kbosctl instances: only one process may hold the lock.
    // `_lock_file` stays alive until process exit, keeping the lock held for
    // the whole operation window (also released on any signal or SIGKILL).
    let _lock_file = match acquire_lock(LOCK_PATH) {
        Ok(fd) => fd,
        Err(msg) => {
            eprintln!("{}", msg);
            exit(1);
        },
    };

    // On any interrupt (Ctrl+C, kill, kill -HUP, ...), ask os-agent to cancel
    // the running operation and exit.
    #[allow(unsafe_code)]
    unsafe {
        for sig in CANCEL_SIGNALS {
            let _ = sigaction(
                *sig,
                &SigAction::new(SigHandler::Handler(handle_sigint), SaFlags::empty(), SigSet::empty()),
            );
        }
    }

    let client = Client::new(SOCK_PATH);

    let req = match cli.command {
        Commands::Upgrade { os_image, configs, skip_tls, reboot } => CmdRequest {
            version: String::new(),
            image_url: String::new(),
            certs: CertsInfo { ca_cert: String::new(), client_cert: String::new(), client_key: String::new() },
            oci_image: os_image,
            configs: configs
                .chunks(2)
                .map(|chunk| ConfigItem { src: chunk[0].clone(), dst: chunk[1].clone() })
                .collect(),
            skip_tls,
            reboot,
            is_rollback: false,
        },
        Commands::Rollback { reboot } => CmdRequest {
            version: String::new(),
            image_url: String::new(),
            certs: CertsInfo { ca_cert: String::new(), client_cert: String::new(), client_key: String::new() },
            oci_image: String::new(),
            configs: vec![],
            skip_tls: false,
            reboot,
            is_rollback: true,
        },
    };

    let method = PrepareCmdMethod::new(req);
    match method.call(&client) {
        Ok(resp) => {
            println!("Operation completed: {:?}", resp.status);
            exit(0);
        },
        Err(e) => {
            eprintln!("Operation failed: {:?}", e);
            exit(1);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_excludes_second_instance() {
        let lock_path = std::env::temp_dir()
            .join(format!("kbosctl-test-{}.lock", std::process::id()))
            .to_str()
            .unwrap()
            .to_string();
        let _f1 = acquire_lock(&lock_path).unwrap();
        let r2 = acquire_lock(&lock_path);
        assert!(r2.is_err());
        drop(_f1);
        let _ = std::fs::remove_file(&lock_path);
    }
}
