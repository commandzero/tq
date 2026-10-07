//! Process-level JSON color contracts and environment precedence.

use std::process::{Command, Output, Stdio};

fn run(args: &[&str], colors: Option<&str>, no_color: bool) -> Output {
    run_with_palettes(args, None, colors, no_color)
}

fn run_with_palettes(
    args: &[&str],
    tq_colors: Option<&str>,
    jq_colors: Option<&str>,
    no_color: bool,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tq"));
    command
        .args(args)
        .env("NO_COLOR", if no_color { "1" } else { "" })
        .env_remove("HOME")
        .env_remove("JQ_LIBRARY_PATH");
    if let Some(colors) = tq_colors {
        command.env("TQ_COLORS", colors);
    } else {
        command.env_remove("TQ_COLORS");
    }
    if let Some(colors) = jq_colors {
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
fn forced_color_uses_the_tq_default_number_style() {
    let output = run(&["-n", "-C", "--output-format", "json", "1"], None, false);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\x1b[0;35m1\x1b[0m\n");
}

#[test]
fn jq_colors_controls_each_json_value_style() {
    let colors = "1;31:1;31:1;31:1;31:1;31:1;31:1;31:1;31";
    assert_eq!(colors.split(':').count(), 8);
    let output = run(
        &["-n", "-C", "--output-format", "json", "1"],
        Some(colors),
        false,
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\x1b[1;31m1\x1b[0m\n");
}

#[test]
fn tq_colors_overrides_jq_colors_across_output_formats() {
    let tq_colors = "1;32:1;32:1;32:1;32:1;32:1;32:1;32";
    let jq_colors = "1;31:1;31:1;31:1;31:1;31:1;31:1;31:1;31";
    for format in ["json", "toon", "yaml", "jsonl", "toon-seq"] {
        let args = ["-n", "-C", "-o", format, "1"];
        let expected = run(&args, Some(tq_colors), false);
        for fallback in [None, Some(jq_colors)] {
            let output = run_with_palettes(&args, Some(tq_colors), fallback, false);
            assert!(output.status.success(), "format {format}");
            assert!(output.stderr.is_empty(), "format {format}");
            assert_eq!(output.stdout, expected.stdout, "format {format}");
            assert!(output.stdout.windows(7).any(|bytes| bytes == b"\x1b[1;32m"));
        }
    }
}

#[test]
fn invalid_tq_colors_selects_defaults_without_using_jq_colors() {
    let jq_colors = "1;31:1;31:1;31:1;31:1;31:1;31:1;31:1;31";
    for tq_colors in ["", "invalid", "31:32"] {
        let output = run_with_palettes(
            &["-n", "-C", "-o", "json", "1"],
            Some(tq_colors),
            Some(jq_colors),
            false,
        );
        assert!(output.status.success());
        assert_eq!(output.stdout, b"\x1b[0;35m1\x1b[0m\n");
    }
}

#[test]
fn tq_colors_does_not_enable_color_or_override_monochrome() {
    let colors = "1;32:1;32:1;32:1;32:1;32:1;32:1;32:1;32";
    for args in [
        vec!["-n", "-o", "json", "1"],
        vec!["-n", "-C", "-M", "-o", "json", "1"],
    ] {
        let output = run_with_palettes(&args, Some(colors), None, false);
        assert!(output.status.success());
        assert_eq!(output.stdout, b"1\n");
    }
}

#[test]
fn seven_entry_jq_colors_uses_the_number_style_for_object_keys() {
    let colors = "1;31:1;32:1;33:1;34:1;35:1;36:1;37";
    assert_eq!(colors.split(':').count(), 7);
    let output = run(
        &["-n", "-C", "-c", "--output-format", "json", "{\"z\":1}"],
        Some(colors),
        false,
    );
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"\x1b[1;37m{\x1b[0m\x1b[1;37m\"\x1b[0m\x1b[1;34mz\x1b[0m\x1b[1;37m\"\x1b[0m\x1b[1;37m:\x1b[0m\x1b[1;34m1\x1b[0m\x1b[1;37m}\x1b[0m\n"
    );
}

#[test]
fn empty_container_delimiters_use_structural_color() {
    let output = run(&["-n", "-C", "--output-format", "json", "[]"], None, false);
    assert!(output.status.success());
    assert_eq!(output.stdout, b"\x1b[0;90m[\x1b[0m\x1b[0;90m]\x1b[0m\n");
}

#[test]
fn explicit_color_overrides_no_color_and_last_flag_wins() {
    let forced = run(&["-n", "-C", "--output-format", "json", "1"], None, true);
    assert_eq!(forced.stdout, b"\x1b[0;35m1\x1b[0m\n");
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
    assert_eq!(reenabled.stdout, b"\x1b[0;35m1\x1b[0m\n");
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
        .env_remove("TQ_COLORS")
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
            .windows(b"\x1b[0;39mnull\x1b[0m".len())
            .any(|window| window == b"\x1b[0;39mnull\x1b[0m"),
        "pty output did not preserve tq's null palette: {:?}",
        output.stdout
    );
}
