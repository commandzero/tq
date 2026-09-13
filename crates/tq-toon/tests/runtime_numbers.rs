//! Computed non-finite numbers project to valid TOON at output boundaries.

use tq_core::{Number, Value};
use tq_toon::{ArrayPreparationConfig, PreparedArray, WriterConfig, encode};

#[test]
fn prepared_output_projects_nan_in_memory_and_after_spilling() {
    for memory_threshold_bytes in [0, 4096] {
        let mut prepared = PreparedArray::new(ArrayPreparationConfig {
            memory_threshold_bytes,
            ..ArrayPreparationConfig::default()
        });
        prepared
            .push(&Value::Number(Number::from_runtime_f64(f64::NAN)))
            .unwrap();
        let mut output = Vec::new();
        prepared
            .write_to(&mut output, WriterConfig::default())
            .unwrap();
        assert_eq!(output, b"[1]: null");
    }
    let value = Value::array([Value::Number(Number::from_runtime_f64(f64::NAN))]);
    assert_eq!(encode(&value, WriterConfig::default()), "[1]: null");
}

#[test]
fn prepared_nested_values_project_nan_without_invalid_number_replay() {
    let mut prepared = PreparedArray::new(ArrayPreparationConfig::default());
    let value = Value::array([Value::Number(Number::from_runtime_f64(f64::NAN))]);
    prepared.push(&value).unwrap();
    let mut output = Vec::new();
    prepared
        .write_to(&mut output, WriterConfig::default())
        .unwrap();
    assert_eq!(output, b"[1]:\n  - [1]: null");
}
