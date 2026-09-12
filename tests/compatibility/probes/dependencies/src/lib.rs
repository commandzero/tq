#![forbid(unsafe_code)]

use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

const PINNED_JQ: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../target/reference-build/jq/jq"
);
const PINNED_JQ_SHA256: &str = "a9fe3ea2f86dfc72f6728417521ec9067b343277152b114f4e98d8cb0e263603";

struct MathBoundaryCase {
    name: &'static str,
    query: &'static str,
    inputs: &'static [(&'static str, f64)],
    apply: fn(f64) -> f64,
}

const ACOS_INPUTS: &[(&str, f64)] = &[
    ("-2", -2.0),
    ("-1", -1.0),
    ("-0.9999999999999999", -0.9999999999999999),
    ("-0.5", -0.5),
    // jq's literal -0 is normalized before this pure libm comparison.
    ("-0", 0.0),
    ("0", 0.0),
    ("0.5", 0.5),
    ("0.9999999999999999", 0.9999999999999999),
    ("1", 1.0),
    ("2", 2.0),
    ("nan", f64::NAN),
    ("infinite", f64::INFINITY),
    ("-infinite", f64::NEG_INFINITY),
];

const EXP_INPUTS: &[(&str, f64)] = &[
    ("-infinite", f64::NEG_INFINITY),
    ("-1000", -1000.0),
    ("-745", -745.0),
    ("-744", -744.0),
    ("-709", -709.0),
    ("-100", -100.0),
    ("-1", -1.0),
    // jq's literal -0 is normalized before this pure libm comparison.
    ("-0", 0.0),
    ("0", 0.0),
    ("1", 1.0),
    ("10", 10.0),
    ("100", 100.0),
    ("709", 709.0),
    ("710", 710.0),
    ("1000", 1000.0),
    ("infinite", f64::INFINITY),
    ("nan", f64::NAN),
];

const ERFC_INPUTS: &[(&str, f64)] = &[
    ("-infinite", f64::NEG_INFINITY),
    ("-30", -30.0),
    ("-10", -10.0),
    ("-2", -2.0),
    ("-1", -1.0),
    // jq's literal -0 is normalized before this pure libm comparison.
    ("-0", 0.0),
    ("0", 0.0),
    ("0.5", 0.5),
    ("1", 1.0),
    ("2", 2.0),
    ("5", 5.0),
    ("10", 10.0),
    ("20", 20.0),
    ("26", 26.0),
    ("27", 27.0),
    ("30", 30.0),
    ("infinite", f64::INFINITY),
    ("nan", f64::NAN),
];

const TGAMMA_INPUTS: &[(&str, f64)] = &[
    ("-infinite", f64::NEG_INFINITY),
    ("-10", -10.0),
    ("-2.5", -2.5),
    ("-1", -1.0),
    ("-0.5", -0.5),
    // jq's literal -0 is normalized before this pure libm comparison.
    ("-0", 0.0),
    ("0", 0.0),
    (r#"("-0.0"|tonumber)"#, -0.0),
    ("0.5", 0.5),
    ("1", 1.0),
    ("1.5", 1.5),
    ("2", 2.0),
    ("10", 10.0),
    ("100", 100.0),
    ("171", 171.0),
    ("172", 172.0),
    ("infinite", f64::INFINITY),
    ("nan", f64::NAN),
];

const Y0_INPUTS: &[(&str, f64)] = &[
    ("-1", -1.0),
    ("0", 0.0),
    ("1e-300", 1e-300),
    ("1e-320", 1e-320),
    ("1e-10", 1e-10),
    ("0.5", 0.5),
    ("0.9999999999999999", 0.9999999999999999),
    ("1", 1.0),
    ("2", 2.0),
    ("10", 10.0),
    ("1e10", 1e10),
    ("infinite", f64::INFINITY),
    ("nan", f64::NAN),
];

const YN_INPUTS: &[(&str, f64)] = Y0_INPUTS;

fn yn_zero(value: f64) -> f64 {
    libm::yn(0, value)
}

fn projected(value: f64) -> Option<f64> {
    if value.is_nan() {
        None
    } else if value.is_infinite() {
        Some(value.signum() * f64::MAX)
    } else {
        Some(value)
    }
}

fn json_text(value: Option<f64>) -> String {
    match value {
        Some(value) => serde_json::to_string(&value).expect("finite value is JSON-safe"),
        None => "null".to_owned(),
    }
}

fn ordered_bits(value: f64) -> u64 {
    let bits = value.to_bits();
    if bits >> 63 == 0 {
        bits | (1 << 63)
    } else {
        !bits
    }
}

fn ulp_distance(left: f64, right: f64) -> Option<u64> {
    if !left.is_finite() || !right.is_finite() {
        return None;
    }
    Some(ordered_bits(left).abs_diff(ordered_bits(right)))
}

fn run_math_boundary_case(case: MathBoundaryCase) {
    let expression = case
        .inputs
        .iter()
        .map(|(input, _)| *input)
        .collect::<Vec<_>>()
        .join(",");
    let query = format!("[{}] | map({})", expression, case.query);
    let output = Command::new(PINNED_JQ)
        .args(["-n", "-c", &query])
        .output()
        .expect("pinned jq must be executable");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{} jq status {:?}, stderr {:?}",
        case.name,
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{} jq stderr {:?}",
        case.name,
        String::from_utf8_lossy(&output.stderr)
    );
    let oracle: Vec<Value> = serde_json::from_slice(&output.stdout).expect("jq JSON output");
    assert_eq!(
        oracle.len(),
        case.inputs.len(),
        "{} result count",
        case.name
    );

    println!(
        "oracle\t{}\t{}\t{}\t{}\t{}\t{}",
        case.name,
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim_end(),
        String::from_utf8_lossy(&output.stderr).trim_end(),
        query,
        PINNED_JQ_SHA256
    );
    for (index, (label, input)) in case.inputs.iter().enumerate() {
        let jq_value = oracle.get(index).expect("validated result count");
        let jq_number = jq_value.as_f64();
        let safe_number = projected((case.apply)(*input));
        let ulp = match (jq_number, safe_number) {
            (Some(left), Some(right)) => ulp_distance(left, right)
                .map_or_else(|| "-".to_owned(), |distance| distance.to_string()),
            _ => "-".to_owned(),
        };
        println!(
            "sample\t{}\t{}\t{}\t{}\t{}\t{}",
            case.name,
            label,
            serde_json::to_string(jq_value).expect("jq value serializes"),
            json_text(safe_number),
            ulp,
            index
        );
    }
}

#[test]
fn pinned_jq_math_boundary_probe_records_function_specific_ulp_observations() {
    let digest = Sha256::digest(std::fs::read(PINNED_JQ).expect("pinned jq bytes"));
    let digest = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(digest, PINNED_JQ_SHA256);
    println!("columns\toracle\tfunction\tstatus\tstdout\tstderr\tquery\tjq_sha256");
    println!("columns\tsample\tfunction\tinput\tjq_value\tsafe_libm_value\tulp_distance\tindex");
    println!(
        "metadata\tarch\t{}\tos\t{}\tlibm\t0.2.16",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    for case in [
        MathBoundaryCase {
            name: "acos",
            query: "acos",
            inputs: ACOS_INPUTS,
            apply: libm::acos,
        },
        MathBoundaryCase {
            name: "exp",
            query: "exp",
            inputs: EXP_INPUTS,
            apply: libm::exp,
        },
        MathBoundaryCase {
            name: "erfc",
            query: "erfc",
            inputs: ERFC_INPUTS,
            apply: libm::erfc,
        },
        MathBoundaryCase {
            name: "tgamma",
            query: "tgamma",
            inputs: TGAMMA_INPUTS,
            apply: libm::tgamma,
        },
        MathBoundaryCase {
            name: "y0",
            query: "y0",
            inputs: Y0_INPUTS,
            apply: libm::y0,
        },
        MathBoundaryCase {
            name: "yn",
            query: "yn(0; .)",
            inputs: YN_INPUTS,
            apply: yn_zero,
        },
    ] {
        run_math_boundary_case(case);
    }
}

#[test]
fn libm_inventory_symbols_compile() {
    let x = 1.25_f64;
    let y = 2.0_f64;
    let _ = [
        libm::acos(x),
        libm::acosh(x),
        libm::asin(x),
        libm::asinh(x),
        libm::atan(x),
        libm::atanh(0.25),
        libm::cbrt(x),
        libm::ceil(x),
        libm::cos(x),
        libm::cosh(x),
        libm::erf(x),
        libm::erfc(x),
        libm::exp(x),
        libm::exp10(x),
        libm::exp2(x),
        libm::expm1(x),
        libm::fabs(x),
        libm::floor(x),
        libm::j0(x),
        libm::j1(x),
        libm::lgamma(x),
        libm::log(x),
        libm::log10(x),
        libm::log1p(x),
        libm::log2(x),
        libm::rint(x),
        libm::round(x),
        libm::sin(x),
        libm::sinh(x),
        libm::sqrt(x),
        libm::tan(x),
        libm::tanh(x),
        libm::tgamma(x),
        libm::trunc(x),
        libm::y0(x),
        libm::y1(x),
        libm::atan2(x, y),
        libm::copysign(x, -y),
        libm::fdim(x, y),
        libm::fmax(x, y),
        libm::fmin(x, y),
        libm::fmod(x, y),
        libm::hypot(x, y),
        libm::ldexp(x, 3),
        libm::nextafter(x, y),
        libm::pow(x, y),
        libm::remainder(x, y),
        libm::scalbn(x, 3),
        libm::yn(0, x),
        libm::jn(0, x),
    ];
    let _ = libm::fma(x, y, 3.0);
    let _ = libm::frexp(x);
    let _ = libm::modf(x);
}

#[test]
fn libm_fma_is_fused_and_signed_zero_is_preserved() {
    let z = libm::fma(1.0 + f64::EPSILON, 1.0 - f64::EPSILON, -1.0);
    assert_eq!(z.to_bits(), (-f64::EPSILON * f64::EPSILON).to_bits());
    assert!(libm::copysign(0.0, -1.0).is_sign_negative());
}

#[test]
fn fancy_regex_probes_semantics_and_budget() {
    let mut builder = fancy_regex::RegexBuilder::new("(a|aa)");
    builder.backtrack_limit(10);
    let re = builder.build().unwrap();
    assert_eq!(re.find("aa").unwrap().unwrap().as_str(), "a");

    let scoped = fancy_regex::Regex::new("(?i:a)b").unwrap();
    assert!(scoped.is_match("Ab").unwrap());

    let unicode = fancy_regex::Regex::new(r"\p{Letter}+").unwrap();
    assert_eq!(unicode.find("élan").unwrap().unwrap().as_str(), "élan");

    let empty = fancy_regex::Regex::new("a*").unwrap();
    assert_eq!(
        empty
            .find_iter("b")
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .len(),
        2
    );

    let hostile = fancy_regex::RegexBuilder::new(r"(?i)(a|b|ab)*(?>c)")
        .backtrack_limit(1)
        .seek(false)
        .build()
        .unwrap();
    assert!(matches!(
        hostile.is_match("abababababababababababababababababababababababababababab"),
        Err(fancy_regex::Error::RuntimeError(
            fancy_regex::RuntimeError::BacktrackLimitExceeded
        ))
    ));
}

#[test]
fn onig_safe_wrapper_probes_longest_and_limits_without_unsafe() {
    let re = onig::Regex::with_options(
        "(a|aa)",
        onig::RegexOptions::REGEX_OPTION_FIND_LONGEST,
        &onig::Syntax::default(),
    )
    .unwrap();
    let mut region = onig::Region::new();
    let mut param = onig::MatchParam::default();
    param.set_match_stack_limit(1024);
    param.set_retry_limit_in_match(10_000);
    let found = re
        .search_with_param(
            "aa",
            0,
            2,
            onig::SearchOptions::SEARCH_OPTION_NONE,
            Some(&mut region),
            param,
        )
        .unwrap();
    assert_eq!(found, Some(0));
    assert_eq!(region.pos(0), Some((0, 2)));
}
