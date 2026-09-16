//! In-process runner, including query compilation and output framing.
use criterion::{BenchmarkId, Throughput, criterion_group, criterion_main};
use std::{hint::black_box, io::Cursor};
use tq_cli::{ExitStatus, parse_args, run_with_io};
#[path = "../../../benchmarks/micro/support.rs"]
mod support;

fn benchmarks(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("event-stream/runner");
    for shape in support::SHAPES {
        for size in support::SIZES {
            let fixture = support::Fixture::new(shape, size);
            for (input, bytes) in [("json", &fixture.json), ("toon", &fixture.toon)] {
                for output in ["json", "toon"] {
                    let mut arguments = vec![
                        "--stream",
                        "--input-format",
                        input,
                        "--output-format",
                        output,
                    ];
                    if output == "toon" {
                        arguments.push("--unframed");
                    } else {
                        arguments.push("--compact-output");
                    }
                    arguments.push(support::QUERY);
                    let command = parse_args(arguments).unwrap();
                    let mut actual = Vec::new();
                    let mut errors = Vec::new();
                    assert_eq!(
                        run_with_io(
                            command.clone(),
                            &mut Cursor::new(bytes),
                            &mut actual,
                            &mut errors
                        )
                        .unwrap(),
                        ExitStatus::Success
                    );
                    assert_eq!(errors, [] as [u8; 0]);
                    assert_eq!(actual, support::Fixture::output(output));
                    group.throughput(Throughput::Bytes(bytes.len() as u64));
                    group.bench_function(
                        BenchmarkId::new(format!("{input}-to-{output}/{shape}"), size),
                        |b| {
                            b.iter(|| {
                                let mut sink = support::ByteSink::default();
                                let mut errors = support::ByteSink::default();
                                black_box(
                                    run_with_io(
                                        black_box(command.clone()),
                                        &mut Cursor::new(black_box(bytes)),
                                        &mut sink,
                                        &mut errors,
                                    )
                                    .unwrap(),
                                );
                                black_box((
                                    sink.bytes,
                                    sink.checksum,
                                    errors.bytes,
                                    errors.checksum,
                                ));
                            });
                        },
                    );
                }
            }
        }
    }
    group.finish();
}
criterion_group! { name = benches; config = support::configuration(); targets = benchmarks }
criterion_main!(benches);
