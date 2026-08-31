//! Byte-equivalence contracts between structural transcode and document output.

use std::{
    io::{self, BufReader, Cursor, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tq_core::SourceId;
use tq_formats::{decode_json, decode_json_events};
use tq_toon::{
    ArrayPreparationConfig, Decoder, DecoderConfig, Delimiter, DuplicateKeyPolicy,
    PreparationArena, PreparationLimits, PublicationBuffer, TranscodeCommitment, TranscodeConsumer,
    WriterConfig, write_sequence, write_unframed,
};

fn preparation(memory: usize, spool: u64) -> (ArrayPreparationConfig, PreparationArena) {
    let config = ArrayPreparationConfig {
        memory_threshold_bytes: memory,
        maximum_spool_bytes: spool,
        spool_directory: std::env::temp_dir(),
        allow_spool: true,
    };
    let arena = PreparationArena::new(PreparationLimits {
        memory_bytes: memory,
        spool_bytes: spool,
        output_bytes: spool,
        ..PreparationLimits::default()
    });
    (config, arena)
}

fn transcode_json(
    input: &[u8],
    writer: WriterConfig,
    unframed: bool,
    memory: usize,
    spool: u64,
) -> Result<Vec<u8>, String> {
    let (preparation, arena) = preparation(memory, spool);
    if unframed {
        let publication = PublicationBuffer::new(preparation.clone(), arena.clone());
        let mut consumer = TranscodeConsumer::new(
            publication,
            writer,
            preparation,
            arena,
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::AtomicUnframed,
        );
        decode_json_events(input, SourceId::new(1), &mut consumer)?;
        let documents = consumer.documents();
        let mut publication = consumer.into_inner();
        let mut output = Vec::new();
        publication
            .publish_single(&mut output, documents)
            .map_err(|error| error.to_string())?;
        Ok(output)
    } else {
        let mut consumer = TranscodeConsumer::new(
            Vec::new(),
            writer,
            preparation,
            arena,
            DuplicateKeyPolicy::Reject,
            TranscodeCommitment::DirectSequence,
        );
        decode_json_events(input, SourceId::new(1), &mut consumer)?;
        Ok(consumer.into_inner())
    }
}

fn document_json(input: &[u8], writer: WriterConfig, unframed: bool) -> Vec<u8> {
    let documents = decode_json(input, "fixture").unwrap();
    let values = documents.iter().map(|document| &document.value);
    let mut output = Vec::new();
    if unframed {
        write_unframed(&mut output, values, writer).unwrap();
    } else {
        write_sequence(&mut output, values, writer).unwrap();
    }
    output
}

#[test]
fn json_transcode_matches_document_for_contract_fixtures_and_delimiters() {
    let fixtures = [
        br#"{"z":9007199254740993,"a":1,"x":2}"#.as_slice(),
        br#"{"empty_object":{},"empty_array":[],"nested":{"b":true}}"#.as_slice(),
        br#"[1,"a,b",null,{"id":1,"name":"Ada"}]"#.as_slice(),
    ];
    for delimiter in [Delimiter::Comma, Delimiter::Tab, Delimiter::Pipe] {
        let writer = WriterConfig {
            delimiter,
            ..WriterConfig::default()
        };
        for input in fixtures {
            for unframed in [false, true] {
                let transcode =
                    transcode_json(input, writer, unframed, 1024 * 1024, 1024 * 1024).unwrap();
                assert_eq!(transcode, document_json(input, writer, unframed));
            }
        }
    }
}

#[test]
fn toon_structural_transcode_matches_document_output() {
    let input = b"z: 1\nitems[2]{id,name}:\n  1,Ada\n  2,Bob";
    let (preparation, arena) = preparation(1024 * 1024, 1024 * 1024);
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation,
        arena,
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    Decoder::new(
        BufReader::new(Cursor::new(input)),
        SourceId::new(1),
        DecoderConfig::default(),
    )
    .decode_into(&mut consumer)
    .unwrap();
    let transcoded = consumer.into_inner();

    let value = tq_toon::decode_to_value(
        Cursor::new(input),
        SourceId::new(1),
        DecoderConfig::default(),
    )
    .unwrap();
    let mut document = Vec::new();
    write_sequence(&mut document, [&value], WriterConfig::default()).unwrap();
    assert_eq!(transcoded, document);
}

#[test]
fn malformed_unframed_input_and_spool_exhaustion_publish_nothing() {
    let malformed = br#"{"a":1,"b":"#;
    assert!(transcode_json(malformed, WriterConfig::default(), true, 16, 1024).is_err());
    assert!(
        transcode_json(
            br#"{"wide":"abcdefghijklmnopqrstuvwxyz"}"#,
            WriterConfig::default(),
            true,
            0,
            4,
        )
        .is_err()
    );
}

#[test]
fn strict_toon_duplicate_key_does_not_publish_a_partial_sequence_record() {
    let input = b"a: 1\na: 2";
    let (preparation, arena) = preparation(1024, 1024);
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation,
        arena,
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    assert!(
        Decoder::new(
            BufReader::new(Cursor::new(input)),
            SourceId::new(1),
            DecoderConfig::default(),
        )
        .decode_into(&mut consumer)
        .is_err()
    );
    assert_eq!(consumer.into_inner(), [] as [u8; 0]);
}

#[test]
fn json_duplicate_key_is_a_streaming_limitation() {
    let input = br#"{"a":1,"a":2}"#;
    let error = transcode_json(input, WriterConfig::default(), false, 1024, 1024)
        .expect_err("streaming transcode must reject a duplicate key");
    assert!(error.contains("duplicate object key 'a'"));

    let output = transcode_json(input, WriterConfig::default(), true, 1024, 1024);
    assert!(
        output.is_err(),
        "unframed transcode must remain unpublished"
    );
}

#[test]
fn generated_values_match_across_formatting_options() {
    let mut values = vec![
        tq_core::Value::Null,
        tq_core::Value::Bool(false),
        tq_core::Value::string("comma, tab\t pipe| and unicode ☃"),
        tq_core::Value::array(Vec::new()),
        tq_core::Value::object(tq_core::Object::new()),
    ];
    for index in 0..32_u64 {
        let mut object = tq_core::Object::new();
        object.insert(
            format!("key_{index}").into(),
            tq_core::Value::array([
                tq_core::Value::Number(tq_core::Number::parse(&index.to_string()).unwrap()),
                tq_core::Value::Bool(index % 2 == 0),
                tq_core::Value::string(format!("value-{index}")),
            ]),
        );
        object.insert(
            "nested".into(),
            tq_core::Value::array([tq_core::Value::object(object.clone())]),
        );
        values.push(tq_core::Value::object(object));
    }

    for value in values {
        let input = serde_json::to_vec(&value).unwrap();
        for delimiter in [Delimiter::Comma, Delimiter::Tab, Delimiter::Pipe] {
            for indent_size in [1, 2, 4] {
                let writer = WriterConfig {
                    delimiter,
                    indent_size,
                    flatten_depth: 1,
                    ..WriterConfig::default()
                };
                assert_eq!(
                    transcode_json(&input, writer, false, 1024 * 1024, 1024 * 1024).unwrap(),
                    document_json(&input, writer, false)
                );
                assert_eq!(
                    transcode_json(&input, writer, true, 1024 * 1024, 1024 * 1024).unwrap(),
                    document_json(&input, writer, true)
                );
            }
        }
    }
}

struct BrokenPipeWriter;

impl Write for BrokenPipeWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed reader"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn cancellation_and_broken_pipe_stop_structural_transcode() {
    let (preparation, arena) = preparation(32, 1024);
    let cancellation = Arc::new(AtomicBool::new(true));
    let mut cancelled = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation.clone(),
        arena.clone(),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    )
    .with_cancellation(Arc::clone(&cancellation));
    assert!(decode_json_events(b"null".as_slice(), SourceId::new(1), &mut cancelled).is_err());
    cancellation.store(false, Ordering::Relaxed);

    let mut broken = TranscodeConsumer::new(
        BrokenPipeWriter,
        WriterConfig::default(),
        preparation,
        arena,
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    assert!(decode_json_events(b"null".as_slice(), SourceId::new(1), &mut broken).is_err());
}

#[test]
fn one_mib_budget_spools_a_direct_nested_array() {
    const MEMORY: usize = 1024 * 1024;
    let mut input = String::from("{\"items\":[");
    for index in 0..250_000_u64 {
        if index != 0 {
            input.push(',');
        }
        input.push_str(&index.to_string());
    }
    input.push_str("]}");

    let (preparation, arena) = preparation(MEMORY, 32 * 1024 * 1024);
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation,
        arena.clone(),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    decode_json_events(input.as_bytes(), SourceId::new(1), &mut consumer).unwrap();
    let output = consumer.into_inner();
    let observations = arena.observations();

    assert_eq!(
        output,
        document_json(input.as_bytes(), WriterConfig::default(), false)
    );
    assert_eq!(observations.array_preparations, 1);
    assert!(observations.spool_bytes_written > 0);
    assert!(observations.spool_bytes_replayed > 0);
    assert!(observations.memory_high_water_bytes <= MEMORY);
}

#[test]
fn composite_array_spills_before_it_starves_the_next_element() {
    const MEMORY: usize = 1024;
    let mut input = String::from("{\"items\":[");
    for index in 0..32_u64 {
        if index != 0 {
            input.push(',');
        }
        input.push_str("{\"value\":\"");
        input.push_str(&"x".repeat(128));
        input.push_str("\"}");
    }
    input.push_str("]}");

    let (preparation, arena) = preparation(MEMORY, 1024 * 1024);
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation,
        arena.clone(),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    decode_json_events(input.as_bytes(), SourceId::new(1), &mut consumer).unwrap();

    assert_eq!(
        consumer.into_inner(),
        document_json(input.as_bytes(), WriterConfig::default(), false)
    );
    assert!(arena.observations().spool_bytes_written > 0);
    assert!(arena.observations().memory_high_water_bytes <= MEMORY);
}

#[test]
fn one_mib_budget_spills_wide_object_keys_and_still_rejects_duplicates() {
    const MEMORY: usize = 1024 * 1024;
    let mut input = String::from("{");
    for index in 0..40_000_u64 {
        if index != 0 {
            input.push(',');
        }
        input.push_str("\"key_");
        input.push_str(&index.to_string());
        input.push_str("\":null");
    }
    input.push_str(",\"key_0\":true}");

    let (preparation, arena) = preparation(MEMORY, 32 * 1024 * 1024);
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation,
        arena.clone(),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    let error = decode_json_events(input.as_bytes(), SourceId::new(1), &mut consumer)
        .expect_err("the spilled duplicate index must remain exact");
    let observations = arena.observations();

    assert!(error.contains("duplicate object key 'key_0'"));
    assert!(observations.object_index_spills > 0);
    assert!(observations.spool_bytes_written > 0);
    assert!(observations.spool_bytes_replayed > 0);
    assert!(observations.memory_high_water_bytes <= MEMORY);
}

#[test]
fn one_mib_budget_bounds_a_nested_composite_element() {
    const MEMORY: usize = 1024 * 1024;
    let mut input = String::from("[[");
    for index in 0..100_000_u64 {
        if index != 0 {
            input.push(',');
        }
        input.push_str(&index.to_string());
    }
    input.push_str("]]\n");

    let (preparation, arena) = preparation(MEMORY, 32 * 1024 * 1024);
    let mut consumer = TranscodeConsumer::new(
        Vec::new(),
        WriterConfig::default(),
        preparation,
        arena.clone(),
        DuplicateKeyPolicy::Reject,
        TranscodeCommitment::DirectSequence,
    );
    let error = decode_json_events(input.as_bytes(), SourceId::new(1), &mut consumer)
        .expect_err("a transient composite must not bypass the aggregate budget");

    assert!(error.contains("nested container exceeds configured preparation memory limit"));
    assert!(arena.observations().memory_high_water_bytes <= MEMORY);
}
