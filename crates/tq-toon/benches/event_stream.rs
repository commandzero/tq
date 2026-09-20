//! TOON structural event decoding with no DOM retention during timing.
use criterion::{BenchmarkId, Throughput, criterion_group, criterion_main};
use std::{hint::black_box, io::Cursor};
use tq_core::SourceId;
use tq_toon::{Decoder, DecoderConfig};
#[path = "../../../benchmarks/micro/events.rs"]
mod events;
#[path = "../../../benchmarks/micro/support.rs"]
mod support;

fn benchmarks(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("event-stream/decode-events");
    for shape in support::SHAPES {
        for size in support::SIZES {
            let fixture = support::Fixture::new(shape, size);
            let mut check = events::Check::default();
            Decoder::new(
                Cursor::new(&fixture.toon),
                SourceId::new(0),
                DecoderConfig::default(),
            )
            .decode_into(&mut check)
            .unwrap();
            assert_eq!(check.events, events::expected(&fixture.model));
            group.throughput(Throughput::Bytes(fixture.toon.len() as u64));
            group.bench_function(BenchmarkId::new(format!("toon/{shape}"), size), |b| {
                b.iter(|| {
                    let mut sink = events::Consume::default();
                    Decoder::new(
                        Cursor::new(black_box(&fixture.toon)),
                        SourceId::new(0),
                        DecoderConfig::default(),
                    )
                    .decode_into(&mut sink)
                    .unwrap();
                    black_box(sink.count);
                });
            });
        }
    }
    group.finish();
}
criterion_group! { name = benches; config = support::configuration(); targets = benchmarks }
criterion_main!(benches);
