//! Process-level JSON color contracts and environment precedence.

use std::process::{Command, Output, Stdio};

fn run(args: &[&str], colors: Option<&str>, no_color: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command
        .args(args)
        .env("NO_COLOR", if no_color { "1" } else { "" })
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH");
    if let Some(colors) = colors {
        command.env("JQ_COLORS", colors);
    } else {
        command.env_remove("JQ_COLORS");
    }
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.output_with_stdin(b"1\n")
}

trait OutputWithStdin {
    fn output_with_stdin(self, input: &[u8]) -> Output;
}

impl OutputWithStdin for Command {
    fn output_with_stdin(mut self, input: &[u8]) -> Output {
        let mut child = self.spawn().expect("spawn color test process");
        std::io::Write::write_all(&mut child.stdin.take().expect("color test stdin"), input)
            .expect("write color test input");
        child.wait_with_output().expect("collect color test output")
    }
}

#[test]
fn forced_color_uses_the_jq_default_number_style() {
    let output = run(&["-n", "-C", "--output-format", "json", "1"], None, false);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\x1b[0;39m1\x1b[0m\n");
}

#[test]
fn jq_colors_controls_each_json_value_style() {
    let colors = "1;31:1;31:1;31:1;31:1;31:1;31:1;31:1;31";
    let output = run(
        &["-n", "-C", "--output-format", "json", "1"],
        Some(colors),
        false,
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\x1b[1;31m1\x1b[0m\n");
}

#[test]
fn seven_entry_jq_colors_uses_the_number_style_for_object_keys() {
    let colors = "1;31:1;32:1;33:1;34:1;35:1;36:1;37";
    let output = run(
        &["-n", "-C", "-c", "--output-format", "json", "{\"z\":1}"],
        Some(colors),
        false,
    );
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"\x1b[1;37m{\x1b[0m\x1b[1;34m\"z\"\x1b[0m\x1b[1;37m:\x1b[0m\x1b[1;34m1\x1b[0m\x1b[1;37m}\x1b[0m\n"
    );
}

#[test]
fn empty_container_uses_one_colored_json_token() {
    let output = run(&["-n", "-C", "--output-format", "json", "[]"], None, false);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\x1b[1;39m[]\x1b[0m\n");
}

#[test]
fn explicit_color_overrides_no_color_and_last_flag_wins() {
    let forced = run(&["-n", "-C", "--output-format", "json", "1"], None, true);
    assert_eq!(forced.stdout, b"\x1b[0;39m1\x1b[0m\n");
    let disabled = run(
        &["-n", "-C", "-M", "--output-format", "json", "1"],
        None,
        false,
    );
    assert_eq!(disabled.stdout, b"1\n");
    let reenabled = run(
        &["-n", "-M", "-C", "--output-format", "json", "1"],
        None,
        false,
    );
    assert_eq!(reenabled.stdout, b"\x1b[0;39m1\x1b[0m\n");
}

#[cfg(unix)]
#[test]
fn auto_color_uses_a_posix_pty_when_stdout_is_a_terminal() {
    let binary = env!("CARGO_BIN_EXE_tq");
    let probe = std::process::Command::new("script")
        .arg("--version")
        .output()
        .expect("probe POSIX script utility");
    let script_version = String::from_utf8_lossy(&probe.stdout);
    let script_version = format!("{script_version}{}", String::from_utf8_lossy(&probe.stderr));

    let mut command = std::process::Command::new("script");
    command
        .arg("-q")
        .env_remove("NO_COLOR")
        .env_remove("JQ_COLORS")
        .env("TERM", "xterm")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if script_version.contains("util-linux") {
        let command_line = format!(
            "'{}' -n --output-format json .",
            binary.replace('\'', "'\\''")
        );
        command.args(["-c", &command_line, "/dev/null"]);
    } else {
        command.args(["/dev/null", binary, "-n", "--output-format", "json", "."]);
    }

    let output = command.output().expect("run tq inside a POSIX pty");
    assert!(
        output.status.success(),
        "script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output
            .stdout
            .windows(b"\x1b[0;90mnull\x1b[0m".len())
            .any(|window| window == b"\x1b[0;90mnull\x1b[0m"),
        "pty output did not preserve jq's null palette: {:?}",
        output.stdout
    );
}
