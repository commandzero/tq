//! Per-event execution, including VM creation, effects, and destruction.
use criterion::{BenchmarkId, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tq_core::{
    AnalysisContext, ResolveOptions, Vm, VmLimits, analyze_with_context, parse, resolve,
};
#[path = "../../../benchmarks/micro/support.rs"]
mod support;

fn benchmarks(c: &mut criterion::Criterion) {
    support::validate_small_contract();
    let plan = analyze_with_context(
        resolve(parse(support::QUERY).unwrap(), &ResolveOptions::default()).unwrap(),
        AnalysisContext {
            event_input: true,
            whole_input: false,
            automatic_streaming: false,
        },
    )
    .compile()
    .unwrap()
    .event_plan()
    .unwrap();
    let mut group = c.benchmark_group("event-stream/execute-events");
    for shape in support::SHAPES {
        for size in support::SIZES {
            let fixture = support::Fixture::new(shape, size);
            for class in ["accepted", "rejected", "end", "mixed"] {
                let records: Vec<_> = fixture
                    .records
                    .iter()
                    .filter(|record| {
                        let json = record.to_json().unwrap();
                        match class {
                            "accepted" => {
                                json.as_array().unwrap().len() == 2
                                    && json[0].as_array().unwrap().len() == 1
                            }
                            "rejected" => {
                                json.as_array().unwrap().len() == 2
                                    && json[0].as_array().unwrap().len() != 1
                            }
                            "end" => json.as_array().unwrap().len() == 1,
                            _ => true,
                        }
                    })
                    .cloned()
                    .collect();
                let mut actual = Vec::new();
                for record in &records {
                    let mut vm = Vm::new_events(&plan, record.clone(), VmLimits::default());
                    vm.for_each_result(|value| {
                        actual.push(value);
                        true
                    })
                    .unwrap();
                    assert_eq!(vm.take_effects(), [] as [u8; 0]);
                }
                let expected = if class == "accepted" || class == "mixed" {
                    fixture.results.clone()
                } else {
                    Vec::new()
                };
                assert_eq!(actual, expected);
                group.throughput(Throughput::Elements(records.len() as u64));
                group.bench_function(BenchmarkId::new(format!("{class}/{shape}"), size), |b| {
                    b.iter(|| {
                        for record in &records {
                            let mut vm = Vm::new_events(
                                black_box(&plan),
                                black_box(record.clone()),
                                VmLimits::default(),
                            );
                            vm.for_each_result(|value| {
                                black_box(value);
                                true
                            })
                            .unwrap();
                            black_box(vm.take_effects());
                        }
                    });
                });
            }
        }
    }
    group.finish();
}
criterion_group! { name = benches; config = support::configuration(); targets = benchmarks }
criterion_main!(benches);
