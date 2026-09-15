//! Semantic palette and undecorated-byte contracts for native writers.

use tq_core::{
    Number, Value,
    presentation::{ColorPalette, ColorRole},
};
use tq_formats::{NativeFormat, NativeOutputSequence, OutputFormat, OutputOptions};

fn colored_options(format: OutputFormat) -> OutputOptions {
    OutputOptions {
        format,
        color_palette: ColorPalette::from_jq_colors("30:31:32:33:34:35:36:37"),
        ..OutputOptions::default()
    }
    .with_color(true)
}

fn write(format: NativeFormat, value: &Value, options: OutputOptions) -> Vec<u8> {
    let selection = format.select_output(options).unwrap();
    let mut sequence = NativeOutputSequence::new(selection);
    let mut output = Vec::new();
    sequence.write_result(&mut output, value).unwrap();
    sequence.finish(&mut output).unwrap();
    output
}

fn strip_sgr(bytes: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'[') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'm' {
                index += 1;
            }
            index += usize::from(index < bytes.len());
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    output
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn assert_strips_to_plain(format: NativeFormat, value: &Value, options: OutputOptions) {
    let colored = write(format, value, options);
    let plain = write(format, value, OutputOptions::default());
    assert_eq!(strip_sgr(&colored), plain);
    assert!(colored.contains(&0x1b));
}

#[test]
fn json_uses_contextual_quotes_and_preserves_escaped_content() {
    let value: Value =
        serde_json::from_str(r#"{"key":["value", "a\"b"], "number":42, "root":"42"}"#).unwrap();
    let colored = write(
        NativeFormat::Json,
        &value,
        colored_options(OutputFormat::Json),
    );
    let plain = write(NativeFormat::Json, &value, OutputOptions::default());

    assert_eq!(strip_sgr(&colored), plain);
    assert!(contains_bytes(&colored, b"\x1b[36m{\x1b"), "{colored:?}");
    assert!(contains_bytes(&colored, b"\x1b[35m[\x1b"), "{colored:?}");
    assert!(contains_bytes(&colored, b"\x1b[37mkey\x1b"), "{colored:?}");
    assert!(
        contains_bytes(&colored, b"\x1b[34mvalue\x1b"),
        "{colored:?}"
    );
    assert!(
        contains_bytes(&colored, b"\x1b[34m\\\"\x1b[0m"),
        "{colored:?}"
    );
}

#[test]
fn json_lines_and_json_sequence_accept_color_and_preserve_frames() {
    let values = [
        Value::Bool(false),
        Value::Number(Number::parse("42").unwrap()),
    ];
    for format in [NativeFormat::JsonLines, NativeFormat::JsonSequence] {
        let options = colored_options(match format {
            NativeFormat::JsonLines => OutputFormat::JsonLines,
            NativeFormat::JsonSequence => OutputFormat::JsonSequence,
            _ => unreachable!(),
        });
        assert_strips_to_plain(format, &values[0], options.clone());
        let colored = write(format, &values[1], options);
        assert!(
            contains_bytes(&colored, b"\x1b[33m42\x1b[0m"),
            "{colored:?}"
        );
        match format {
            NativeFormat::JsonLines => assert!(colored.ends_with(b"\n")),
            NativeFormat::JsonSequence => assert!(colored.starts_with(b"\x1e")),
            _ => unreachable!(),
        }
    }
}

#[test]
fn seven_slot_palette_uses_number_style_for_object_keys() {
    let value: Value = serde_json::from_str(r#"{"z":1}"#).unwrap();
    let options = OutputOptions {
        format: OutputFormat::Json,
        color_palette: ColorPalette::from_jq_colors("1;31:1;32:1;33:1;34:1;35:1;36:1;37"),
        ..OutputOptions::default()
    }
    .with_color(true);
    assert_eq!(options.color_palette.style(ColorRole::Number), "1;34");
    assert_eq!(options.color_palette.style(ColorRole::Object), "1;37");
    assert_eq!(options.color_palette.style(ColorRole::Key), "1;34");
    let colored = write(NativeFormat::Json, &value, options);
    let plain = write(NativeFormat::Json, &value, OutputOptions::default());
    assert_eq!(strip_sgr(&colored), plain);
    assert!(
        contains_bytes(&colored, b"\x1b[1;34mz\x1b[0m"),
        "{colored:?}"
    );
}

#[test]
fn ascii_json_coloring_escapes_without_changing_plain_bytes() {
    let value = Value::string("café 😀");
    let colored_options = OutputOptions {
        format: OutputFormat::Json,
        ascii_json: true,
        color_palette: ColorPalette::from_jq_colors("30:31:32:33:34:35:36:37"),
        ..OutputOptions::default()
    }
    .with_color(true);
    let colored = write(NativeFormat::Json, &value, colored_options);
    let plain = write(
        NativeFormat::Json,
        &value,
        OutputOptions {
            format: OutputFormat::Json,
            ascii_json: true,
            ..OutputOptions::default()
        },
    );
    assert_eq!(strip_sgr(&colored), plain);
    assert!(contains_bytes(&colored, br"\u00e9"));
    assert!(contains_bytes(&colored, br"\ud83d\ude00"));
}

#[test]
fn root_string_quotes_use_object_style() {
    let value = Value::string("42");
    let colored = write(
        NativeFormat::Json,
        &value,
        colored_options(OutputFormat::Json),
    );
    assert!(
        contains_bytes(&colored, b"\x1b[36m\"\x1b[0m"),
        "{colored:?}"
    );
    assert!(
        contains_bytes(&colored, b"\x1b[34m42\x1b[0m"),
        "{colored:?}"
    );
}

#[test]
fn yaml_and_delimited_writers_color_syntax_without_changing_bytes() {
    let value: Value =
        serde_json::from_str(r#"{"name":"Ada", "quoted":"42", "n":42, "ok":true, "nil":null}"#)
            .unwrap();
    assert_strips_to_plain(
        NativeFormat::Yaml,
        &value,
        colored_options(OutputFormat::Yaml),
    );
    for format in [NativeFormat::Csv, NativeFormat::Tsv] {
        assert_strips_to_plain(
            format,
            &value,
            colored_options(match format {
                NativeFormat::Csv => OutputFormat::Csv,
                NativeFormat::Tsv => OutputFormat::Tsv,
                _ => unreachable!(),
            }),
        );
    }
}

#[test]
fn colored_yaml_strict_conversion_validates_unstyled_bytes() {
    let value: Value = serde_json::from_str(r#"{"n":9007199254740993}"#).unwrap();
    let options = OutputOptions {
        format: OutputFormat::Yaml,
        strict_conversion: true,
        ..OutputOptions::default()
    }
    .with_color(true);
    assert_strips_to_plain(NativeFormat::Yaml, &value, options);
}

#[test]
fn delimited_header_escaped_quotes_keep_the_key_role() {
    let value: Value = serde_json::from_str(r#"{"a\"b":"x\"y"}"#).unwrap();
    let options = OutputOptions {
        format: OutputFormat::Csv,
        color_palette: ColorPalette::from_jq_colors("30:31:32:33:34:35:36:37"),
        ..OutputOptions::default()
    }
    .with_color(true);
    let colored = write(NativeFormat::Csv, &value, options);
    let plain = write(NativeFormat::Csv, &value, OutputOptions::default());
    assert_eq!(strip_sgr(&colored), plain);
    assert!(contains_bytes(&colored, b"\x1b[37ma\x1b[0m"), "{colored:?}");
    assert!(
        contains_bytes(&colored, b"\x1b[37m\"\"\x1b[0m"),
        "{colored:?}"
    );
    assert!(
        contains_bytes(&colored, b"\x1b[34m\"\"\x1b[0m"),
        "{colored:?}"
    );
}

#[test]
fn default_palette_matches_the_shared_roles() {
    let palette = ColorPalette::default();
    assert_eq!(palette.style(ColorRole::False), "0;94");
    assert_eq!(palette.style(ColorRole::True), "0;94");
    assert_eq!(palette.style(ColorRole::Array), "0;90");
    assert_eq!(palette.style(ColorRole::Object), "0;90");
    assert_eq!(palette.style(ColorRole::Key), "0;36");
}

#[test]
fn exact_number_spellings_receive_number_color_in_every_json_writer() {
    for spelling in ["1E+3", "1e+03", "-0.00"] {
        let parsed = Number::parse(spelling).unwrap();
        let preserved = parsed.to_string();
        let number = Value::Number(parsed);
        for (native, format) in [
            (NativeFormat::Json, OutputFormat::Json),
            (NativeFormat::JsonLines, OutputFormat::JsonLines),
            (NativeFormat::JsonSequence, OutputFormat::JsonSequence),
        ] {
            let options = colored_options(format);
            let expected = format!("\x1b[33m{preserved}\x1b[0m");
            let colored = write(native, &number, options.clone());
            assert!(contains_bytes(&colored, expected.as_bytes()), "{colored:?}");
            assert_strips_to_plain(native, &number, options.clone());
            let mut raw = Vec::new();
            tq_formats::write_raw_json_value(&mut raw, &number, Some(&options.color_palette))
                .unwrap();
            assert_eq!(raw, expected.as_bytes());
        }
    }
}

#[test]
fn delimited_row_limits_apply_to_undecorated_logical_rows() {
    let value: Value = serde_json::from_str(r#"{"x":1}"#).unwrap();
    for (native, format) in [
        (NativeFormat::Csv, OutputFormat::Csv),
        (NativeFormat::Tsv, OutputFormat::Tsv),
    ] {
        let mut options = colored_options(format);
        options.delimited_limits.row_bytes = 2;
        let colored = write(native, &value, options.clone());
        assert_eq!(strip_sgr(&colored), b"x\n1\n");
        assert!(colored.len() > 4);
        options.delimited_limits.row_bytes = 1;
        let mut sequence = NativeOutputSequence::new(native.select_output(options).unwrap());
        let mut output = Vec::new();
        assert!(sequence.write_result(&mut output, &value).is_err());
        assert_eq!(output, b"");
    }
}
