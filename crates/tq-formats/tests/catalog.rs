//! Format selection and capability behavior through the public catalog.

use tq_formats::{InputFormat, NativeFormat, OutputFormat};

#[test]
fn catalog_resolves_names_aliases_and_extensions_without_guessing() {
    assert_eq!(
        NativeFormat::from_name("ndjson"),
        Some(NativeFormat::JsonLines)
    );
    assert_eq!(
        NativeFormat::from_extension("JSON5"),
        Some(NativeFormat::Json5)
    );
    assert_eq!(NativeFormat::from_name("auto"), None);
    assert_eq!(NativeFormat::from_extension("unknown"), None);
    assert_eq!(NativeFormat::Json5.descriptor().output, None);
    assert_eq!(
        NativeFormat::Json.descriptor().output,
        Some(OutputFormat::Json)
    );
}

#[test]
fn catalog_covers_every_concrete_input_and_keeps_directional_capabilities() {
    for input in [
        InputFormat::Toon,
        InputFormat::Yaml,
        InputFormat::Json,
        InputFormat::Json5,
        InputFormat::JsonLines,
        InputFormat::ToonSequence,
        InputFormat::JsonSequence,
    ] {
        let native = NativeFormat::from_input(input).expect("concrete selection");
        assert_eq!(native.descriptor().input, input);
        assert_eq!(
            NativeFormat::from_name(native.descriptor().name),
            Some(native)
        );
    }
    assert_eq!(NativeFormat::from_input(InputFormat::Auto), None);
    assert!(NativeFormat::Json.descriptor().events);
    assert!(!NativeFormat::Json5.descriptor().events);
    assert!(!NativeFormat::JsonLines.descriptor().probeable);
}

#[test]
fn probe_json_sequence_commits_only_to_a_leading_separator() {
    use tq_formats::probe_format;
    for bytes in [b"\x1e{}\n".as_slice(), b" \t\n\x1e\xff"] {
        let report = probe_format(bytes, 64).unwrap();
        assert_eq!(report.selected, InputFormat::JsonSequence);
        assert!(report.commitment_bytes <= report.lookahead_bytes);
    }
    let bytes = b"  \x1e{}\n";
    assert_ne!(
        probe_format(bytes, 2).unwrap().selected,
        InputFormat::JsonSequence
    );
    assert_eq!(
        probe_format(bytes, 3).unwrap().selected,
        InputFormat::JsonSequence
    );
    assert_ne!(
        probe_format(b"text\x1e{}\n", 64).unwrap().selected,
        InputFormat::JsonSequence
    );
    assert_eq!(
        NativeFormat::from_extension("JSONSEQ"),
        Some(NativeFormat::JsonSequence)
    );
}
