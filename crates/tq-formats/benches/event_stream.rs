//! JSON event decode, jq record production, and output sequence encoding.
use criterion::{BenchmarkId, Throughput, criterion_group, criterion_main};
use std::{hint::black_box, io::Cursor};
use tq_core::SourceId;
use tq_formats::{JsonEventOptions, decode_json_event_stream};
#[path = "../../../benchmarks/micro/events.rs"]
mod events;
#[path = "../../../benchmarks/micro/formats.rs"]
mod formats;
#[path = "../../../benchmarks/micro/support.rs"]
mod support;

fn benchmarks(c: &mut criterion::Criterion) {
    support::validate_small_contract();
    decode(c);
    records(c);
    encode(c);
}

fn decode(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("event-stream/decode-events");
    for shape in support::SHAPES {
        for size in support::SIZES {
            let fixture = support::Fixture::new(shape, size);
            let mut check = events::Check::default();
            assert_eq!(
                decode_json_event_stream(
                    Cursor::new(&fixture.json),
                    SourceId::new(0),
                    &mut check,
                    JsonEventOptions::default()
                )
                .unwrap(),
                1
            );
            assert_eq!(check.events, events::expected(&fixture.model));
            group.throughput(Throughput::Bytes(fixture.json.len() as u64));
            group.bench_function(BenchmarkId::new(format!("json/{shape}"), size), |b| {
                b.iter(|| {
                    let mut sink = events::Consume::default();
                    black_box(
                        decode_json_event_stream(
                            Cursor::new(black_box(&fixture.json)),
                            SourceId::new(0),
                            &mut sink,
                            JsonEventOptions::default(),
                        )
                        .unwrap(),
                    );
                    black_box(sink.count);
                });
            });
        }
    }
    group.finish();
}

fn records(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("event-stream/stream-records");
    for shape in support::SHAPES {
        for size in support::SIZES {
            let fixture = support::Fixture::new(shape, size);
            for (format, bytes) in [("json", &fixture.json), ("toon", &fixture.toon)] {
                let mut actual = Vec::new();
                formats::stream(format, bytes, |record| {
                    actual.push(record);
                    Ok(())
                });
                assert_eq!(actual, fixture.records);
                group.throughput(Throughput::Elements(fixture.records.len() as u64));
                group.bench_function(BenchmarkId::new(format!("{format}/{shape}"), size), |b| {
                    b.iter(|| {
                        formats::stream(format, black_box(bytes), |record| {
                            black_box(record);
                            Ok(())
                        });
                    });
                });
            }
        }
    }
    group.finish();
}

fn encode(c: &mut criterion::Criterion) {
    // This query emits exactly one fixed-size result regardless of input size.
    // One case per output avoids presenting duplicate timings as size coverage.
    let fixture = support::Fixture::new("tabular", 16);
    let mut group = c.benchmark_group("event-stream/encode-results");
    for format in ["json", "toon"] {
        let mut actual = Vec::new();
        formats::encode(format, &fixture.results, &mut actual);
        assert_eq!(actual, support::Fixture::output(format));
        group.throughput(Throughput::Bytes(actual.len() as u64));
        group.bench_function(BenchmarkId::new(format!("{format}/selected"), 1), |b| {
            b.iter(|| {
                let mut sink = support::ByteSink::default();
                formats::encode(format, black_box(&fixture.results), &mut sink);
                black_box((sink.bytes, sink.checksum));
            });
        });
    }
    group.finish();
}
criterion_group! { name = benches; config = support::configuration(); targets = benchmarks }
criterion_main!(benches);
