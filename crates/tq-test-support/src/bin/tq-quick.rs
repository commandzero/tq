//! Bound an entire post-compilation benchmark stage, including preparation.

use std::{
    env, io,
    path::PathBuf,
    process::{Command, ExitCode},
    time::Duration,
};
#[cfg(unix)]
use std::{fs, os::unix::process::CommandExt};
use tq_test_support::benchmark::quick::{QUICK_WORK_BUDGET_SECONDS, supervise_command};

#[cfg(unix)]
fn clean_install_and_exec(mut args: impl Iterator<Item = std::ffi::OsString>) -> io::Result<i32> {
    let root = PathBuf::from(args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "guard cleanup requires an installation root",
        )
    })?);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "guard cleanup requires -- COMMAND",
        ));
    }
    let executable = args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "guard cleanup requires COMMAND",
        )
    })?;
    let guard = root.join("bin/tq-quick").canonicalize()?;
    if !root.join(".tq-quick-install").is_file() || guard != env::current_exe()?.canonicalize()? {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to clean a guard installation not owned by this process",
        ));
    }
    fs::remove_dir_all(root)?;
    let mut command = Command::new(executable);
    command.args(args);
    Err(command.exec())
}

fn run() -> io::Result<i32> {
    let mut args = env::args_os().skip(1);
    let mut seconds = QUICK_WORK_BUDGET_SECONDS;
    let mut first = args.next();
    #[cfg(unix)]
    if first.as_deref() == Some(std::ffi::OsStr::new("--cleanup-install-root-child")) {
        return clean_install_and_exec(args);
    }
    let mut install_root = None;
    if first.as_deref() == Some(std::ffi::OsStr::new("--cleanup-install-root")) {
        install_root = Some(PathBuf::from(args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--cleanup-install-root requires a path",
            )
        })?));
        first = args.next();
    }
    if first.as_deref() == Some(std::ffi::OsStr::new("--budget-seconds")) {
        let value = args.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--budget-seconds requires an integer from 1 to 50",
            )
        })?;
        seconds = value
            .to_str()
            .and_then(|value| value.parse().ok())
            .filter(|seconds| (1..=QUICK_WORK_BUDGET_SECONDS).contains(seconds))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "--budget-seconds requires an integer from 1 to 50",
                )
            })?;
        first = args.next();
    }
    if first.as_deref() != Some(std::ffi::OsStr::new("--")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: tq-quick [--cleanup-install-root PATH] [--budget-seconds 1..50] -- COMMAND [ARGS...]",
        ));
    }
    let executable = args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "tq-quick requires COMMAND after --",
        )
    })?;
    let mut command = if let Some(root) = install_root {
        #[cfg(unix)]
        {
            let mut command = Command::new(env::current_exe()?);
            command
                .arg("--cleanup-install-root-child")
                .arg(root)
                .arg("--")
                .arg(executable);
            command
        }
        #[cfg(not(unix))]
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "installation cleanup requires Unix",
            ));
        }
    } else {
        Command::new(executable)
    };
    command.args(args);
    supervise_command(&mut command, Duration::from_secs(seconds))
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            eprintln!("tq-quick: {error}");
            ExitCode::from(2)
        }
    }
}
