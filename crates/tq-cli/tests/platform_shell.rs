//! Native shell argument and output contracts.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn tq_binary() -> &'static str {
    env!("CARGO_BIN_EXE_tq")
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, [] as [u8; 0]);
    assert_eq!(output.stdout, b"\"bar\"\n");
}

#[cfg(unix)]
#[test]
fn posix_shell_preserves_quoted_filter_variables_and_paths() {
    let directory = tempfile::tempdir().expect("shell fixture directory");
    let filter = directory.path().join("filter with spaces.jq");
    fs::write(&filter, ".[$name]").expect("shell filter writes");
    let command =
        r#"printf '%s\n' '{"foo":"bar"}' | "$TQ_BIN" --arg name foo -ijson -ojson -c '.[ $name ]'"#;
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(command)
        .env("TQ_BIN", tq_binary())
        .env("FILTER", &filter)
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH")
        .output()
        .expect("run POSIX shell filter");
    assert_success(&output);

    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(r#"printf '%s\n' '{"foo":"bar"}' | "$TQ_BIN" --arg name foo -ijson -ojson -c -f "$FILTER""#)
        .env("TQ_BIN", tq_binary())
        .env("FILTER", &filter)
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH")
        .output()
        .expect("run POSIX shell file filter");
    assert_success(&output);
}

#[test]
fn powershell_preserves_quoted_filter_variables_and_paths_when_available() {
    let powershell = if Path::new("/opt/homebrew/bin/pwsh").is_file() {
        Some(PathBuf::from("/opt/homebrew/bin/pwsh"))
    } else {
        ["pwsh", "powershell.exe", "powershell"]
            .into_iter()
            .find(|candidate| {
                Command::new(candidate)
                    .args(["-NoLogo", "-NoProfile", "-Command", "$null"])
                    .status()
                    .is_ok_and(|status| status.success())
            })
            .map(PathBuf::from)
    };
    let Some(powershell) = powershell else {
        #[cfg(windows)]
        panic!("PowerShell is required for the native Windows shell contract");
        #[cfg(not(windows))]
        eprintln!("PowerShell unavailable: platform shell evidence is unverified");
        #[cfg(not(windows))]
        return;
    };
    let directory = tempfile::tempdir().expect("PowerShell fixture directory");
    let filter = directory.path().join("filter with spaces.jq");
    fs::write(&filter, ".[$name]").expect("PowerShell filter writes");
    let command = r#"$payload = '{"foo":"bar"}'; $payload | & $env:TQ_BIN --arg name foo -ijson -ojson -c '.[ $name ]'"#;
    let output = Command::new(&powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", command])
        .env("TQ_BIN", tq_binary())
        .env("FILTER", &filter)
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH")
        .output()
        .expect("run PowerShell filter");
    assert_success(&output);

    let output = Command::new(&powershell)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"$payload = '{"foo":"bar"}'; $payload | & $env:TQ_BIN --arg name foo -ijson -ojson -c -f $env:FILTER"#,
        ])
        .env("TQ_BIN", tq_binary())
        .env("FILTER", &filter)
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH")
        .output()
        .expect("run PowerShell file filter");
    assert_success(&output);
}

#[cfg(windows)]
#[test]
fn cmd_preserves_quoted_filter_variables_and_paths() {
    let directory = tempfile::tempdir().expect("cmd fixture directory");
    let filter = directory.path().join("filter with spaces.jq");
    fs::write(&filter, ".[ $name ]").expect("cmd filter writes");
    let output = Command::new("cmd.exe")
        .args([
            "/C",
            "echo {\"foo\":\"bar\"}|\"%TQ_BIN%\" --arg name foo -ijson -ojson -c -f \"%FILTER%\"",
        ])
        .env("TQ_BIN", tq_binary())
        .env("FILTER", &filter)
        .output()
        .expect("run cmd filter");
    assert_success(&output);
}

#[cfg(windows)]
#[test]
fn windows_binary_mode_keeps_raw_output_lf_terminated() {
    let output = Command::new(tq_binary())
        .args(["-n", "-b", "-r", "\"line\\nnext\""])
        .output()
        .expect("run Windows binary output");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"line\nnext\n");
    assert!(output.stderr.is_empty());
}
