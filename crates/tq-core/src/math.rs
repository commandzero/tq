//! Safe Rust implementations of jq's scalar mathematical built-ins.
//!
//! This module deliberately works on `f64` values rather than [`crate::Number`]
//! values.  That keeps the mathematical seam faithful to the platform-library
//! contract, including non-finite results, while numeric projection remains a
//! separate integration decision for the evaluator.

use std::{fmt, num::FpCategory};

/// A mathematical result returned by a jq-compatible function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MathResult {
    /// A scalar result.
    Scalar(f64),
    /// A two-element result, used by `frexp` and `modf`.
    Pair([f64; 2]),
}

/// Errors raised before a mathematical operation can be evaluated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MathError {
    /// The name is not part of the supported safe math inventory.
    UnknownFunction,
    /// The number of values supplied to the mathematical seam is invalid.
    Arity {
        /// Function name supplied by the caller.
        function: String,
        /// Number of values required by the operation.
        expected: usize,
        /// Number of values supplied by the caller.
        actual: usize,
    },
    /// A Bessel order is non-finite or too large for bounded execution.
    InvalidOrder { function: String },
}

const MAX_BESSEL_ORDER: f64 = 1_024.0;

impl fmt::Display for MathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFunction => formatter.write_str("unknown mathematical function"),
            Self::Arity {
                function,
                expected,
                actual,
            } => write!(
                formatter,
                "{function} expects {expected} numeric value(s), got {actual}"
            ),
            Self::InvalidOrder { function } => {
                write!(formatter, "{function} requires a bounded finite order")
            }
        }
    }
}

impl std::error::Error for MathError {}

/// Evaluates one function from the jq math inventory.
///
/// The caller supplies exactly the numeric values consumed by the C jq
/// wrapper: unary functions receive one value, binary functions two explicit
/// values, and ternary functions three explicit values.  `frexp` and `modf`
/// return a [`MathResult::Pair`] in the same order as jq's array result.
pub fn evaluate(name: &str, arguments: &[f64]) -> Result<MathResult, MathError> {
    match name {
        // Use the safe platform operation for the observed jq rounding
        // boundary, rather than the independently rounded libm algorithm.
        "acos" => unary(name, arguments, f64::acos),
        "acosh" => unary(name, arguments, libm::acosh),
        "asin" => unary(name, arguments, libm::asin),
        "asinh" => unary(name, arguments, libm::asinh),
        "atan" => unary(name, arguments, libm::atan),
        "atan2" => binary(name, arguments, libm::atan2),
        "atanh" => unary(name, arguments, libm::atanh),
        "cbrt" => unary(name, arguments, libm::cbrt),
        "ceil" => unary(name, arguments, libm::ceil),
        "copysign" => binary(name, arguments, libm::copysign),
        "cos" => unary(name, arguments, libm::cos),
        "cosh" => unary(name, arguments, libm::cosh),
        "drem" | "remainder" => binary(name, arguments, libm::remainder),
        "erf" => unary(name, arguments, libm::erf),
        "erfc" => unary(name, arguments, libm::erfc),
        "exp" => unary(name, arguments, f64::exp),
        "exp10" => unary(name, arguments, libm::exp10),
        "exp2" => unary(name, arguments, libm::exp2),
        "expm1" => unary(name, arguments, libm::expm1),
        "fabs" => unary(name, arguments, libm::fabs),
        "fdim" => binary(name, arguments, libm::fdim),
        "fma" => ternary(name, arguments, libm::fma),
        "fmax" => binary(name, arguments, fmax_value),
        "fmin" => binary(name, arguments, fmin_value),
        "fmod" => binary(name, arguments, libm::fmod),
        "floor" => unary(name, arguments, libm::floor),
        "frexp" => pair(name, arguments, |value| {
            let (fraction, exponent) = libm::frexp(value);
            [fraction, f64::from(exponent)]
        }),
        "gamma" => unary(name, arguments, gamma_value),
        "tgamma" => unary(name, arguments, libm::tgamma),
        "hypot" => binary(name, arguments, libm::hypot),
        "j0" => unary(name, arguments, libm::j0),
        "j1" => unary(name, arguments, libm::j1),
        "jn" => bessel("jn", arguments, libm::jn),
        "ldexp" => scaled("ldexp", arguments),
        "lgamma" => unary(name, arguments, libm::lgamma),
        "log" => unary(name, arguments, libm::log),
        "log10" => unary(name, arguments, libm::log10),
        "log1p" => unary(name, arguments, libm::log1p),
        "log2" => unary(name, arguments, libm::log2),
        "logb" => unary(name, arguments, logb),
        "modf" => pair(name, arguments, |value| {
            let (fraction, integral) = libm::modf(value);
            [fraction, integral]
        }),
        "nearbyint" | "rint" => unary(name, arguments, libm::rint),
        "nextafter" | "nexttoward" => binary(name, arguments, libm::nextafter),
        "pow" => binary(name, arguments, libm::pow),
        "round" => unary(name, arguments, libm::round),
        "scalb" => scaled("scalb", arguments),
        "scalbln" => scaled("scalbln", arguments),
        "significand" => unary(name, arguments, significand),
        "sin" => unary(name, arguments, libm::sin),
        "sinh" => unary(name, arguments, libm::sinh),
        "sqrt" => unary(name, arguments, libm::sqrt),
        "tan" => unary(name, arguments, libm::tan),
        "tanh" => unary(name, arguments, libm::tanh),
        "trunc" => unary(name, arguments, libm::trunc),
        "y0" => unary(name, arguments, libm::y0),
        "y1" => unary(name, arguments, libm::y1),
        "yn" => bessel("yn", arguments, libm::yn),
        _ => Err(MathError::UnknownFunction),
    }
}

fn unary(
    function: &str,
    arguments: &[f64],
    operation: fn(f64) -> f64,
) -> Result<MathResult, MathError> {
    let [value] = arguments else {
        return Err(MathError::Arity {
            function: function.to_owned(),
            expected: 1,
            actual: arguments.len(),
        });
    };
    Ok(MathResult::Scalar(operation(*value)))
}

fn binary(
    function: &str,
    arguments: &[f64],
    operation: fn(f64, f64) -> f64,
) -> Result<MathResult, MathError> {
    let [left, right] = arguments else {
        return Err(MathError::Arity {
            function: function.to_owned(),
            expected: 2,
            actual: arguments.len(),
        });
    };
    Ok(MathResult::Scalar(operation(*left, *right)))
}

fn ternary(
    function: &str,
    arguments: &[f64],
    operation: fn(f64, f64, f64) -> f64,
) -> Result<MathResult, MathError> {
    let [first, second, third] = arguments else {
        return Err(MathError::Arity {
            function: function.to_owned(),
            expected: 3,
            actual: arguments.len(),
        });
    };
    Ok(MathResult::Scalar(operation(*first, *second, *third)))
}

fn scaled(function: &str, arguments: &[f64]) -> Result<MathResult, MathError> {
    let [value, exponent] = arguments else {
        return Err(MathError::Arity {
            function: function.to_owned(),
            expected: 2,
            actual: arguments.len(),
        });
    };
    match function {
        // jq's C wrapper passes a JSON double to the C `int`/`long` argument
        // of these functions; conversion truncates toward zero. Saturating
        // to libm's i32 API is safe because either extreme has already
        // overflowed or underflowed every finite f64 input.
        "ldexp" | "scalbln" => Ok(MathResult::Scalar(libm::scalbn(
            *value,
            scale_exponent_saturating(*exponent),
        ))),
        // The platform `scalb` accepts a floating exponent. Its C behavior
        // truncates finite fractions and has defined non-finite projections.
        "scalb" => Ok(MathResult::Scalar(scalb_value(*value, *exponent))),
        _ => unreachable!("scaled called for an unknown function"),
    }
}

fn pair(
    function: &str,
    arguments: &[f64],
    operation: fn(f64) -> [f64; 2],
) -> Result<MathResult, MathError> {
    let [value] = arguments else {
        return Err(MathError::Arity {
            function: function.to_owned(),
            expected: 1,
            actual: arguments.len(),
        });
    };
    Ok(MathResult::Pair(operation(*value)))
}

// The finite/order bound above makes this C-wrapper-compatible truncating
// conversion provably fit in i32; Rust has no checked f64-to-integer cast.
#[allow(clippy::cast_possible_truncation)]
fn bessel(
    function: &str,
    arguments: &[f64],
    operation: fn(i32, f64) -> f64,
) -> Result<MathResult, MathError> {
    let [order, value] = arguments else {
        return Err(MathError::Arity {
            function: function.to_owned(),
            expected: 2,
            actual: arguments.len(),
        });
    };
    // libm's integer-order recurrence is work proportional to the order.  A
    // fixed cap keeps this public seam bounded even before the VM supplies its
    // configurable resource limits.  Fractional finite values follow the C
    // wrapper's truncating conversion; non-finite and out-of-range orders are
    // rejected instead of silently becoming zero or triggering a huge loop.
    if !order.is_finite() || order.abs() > MAX_BESSEL_ORDER {
        return Err(MathError::InvalidOrder {
            function: function.to_owned(),
        });
    }
    Ok(MathResult::Scalar(operation(*order as i32, *value)))
}

// The range checks make this truncating conversion provably fit in i32.
#[allow(clippy::cast_possible_truncation)]
fn scale_exponent_saturating(value: f64) -> i32 {
    if value.is_nan() {
        return 0;
    }
    let truncated = value.trunc();
    if truncated >= f64::from(i32::MAX) {
        i32::MAX
    } else if truncated <= f64::from(i32::MIN) {
        i32::MIN
    } else {
        truncated as i32
    }
}

#[cfg(target_os = "linux")]
fn gamma_value(value: f64) -> f64 {
    // The pinned Linux jq build resolves gamma to the platform lgamma
    // symbol; keep this target-specific rather than treating gamma and
    // tgamma as portable aliases.
    libm::lgamma(value)
}

#[cfg(not(target_os = "linux"))]
fn gamma_value(value: f64) -> f64 {
    libm::tgamma(value)
}

#[cfg(target_os = "linux")]
fn scalb_value(value: f64, exponent: f64) -> f64 {
    // glibc's scalb contract rejects fractional exponents, while its
    // infinite-exponent cases follow the IEEE zero/infinity products. The
    // pinned Linux jq build projects NaN as null and infinity as finite max.
    if value.is_nan() || exponent.is_nan() {
        return f64::NAN;
    }
    if exponent.is_infinite() {
        if exponent.is_sign_positive() {
            return if value == 0.0 {
                f64::NAN
            } else {
                value.signum() * f64::INFINITY
            };
        }
        return if value.is_infinite() {
            f64::NAN
        } else {
            value.signum() * 0.0
        };
    }
    if exponent.fract() != 0.0 {
        return f64::NAN;
    }
    libm::scalbn(value, scale_exponent_saturating(exponent))
}

#[cfg(not(target_os = "linux"))]
fn scalb_value(value: f64, exponent: f64) -> f64 {
    if value.is_nan() || exponent.is_nan() {
        return f64::NAN;
    }
    if exponent.is_infinite() {
        if value == 0.0 {
            return value;
        }
        if value.is_infinite() {
            return value;
        }
        return if exponent.is_sign_positive() {
            value.signum() * f64::INFINITY
        } else {
            value.signum() * 0.0
        };
    }
    libm::scalbn(value, scale_exponent_saturating(exponent))
}

fn logb(value: f64) -> f64 {
    match value.classify() {
        FpCategory::Zero => f64::NEG_INFINITY,
        FpCategory::Infinite => f64::INFINITY,
        FpCategory::Nan => f64::NAN,
        FpCategory::Normal | FpCategory::Subnormal => f64::from(libm::ilogb(value)),
    }
}

fn significand(value: f64) -> f64 {
    let (fraction, _) = libm::frexp(value);
    fraction * 2.0
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn fmax_value(left: f64, right: f64) -> f64 {
    if left == 0.0 && right == 0.0 {
        // The pinned x86_64 GNU/Linux jq/glibc contract preserves the first
        // operand's sign for equal signed zeros. This is target-specific;
        // other targets use the safe-library canonical rule below.
        left
    } else {
        libm::fmax(left, right)
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn fmax_value(left: f64, right: f64) -> f64 {
    if left == 0.0 && right == 0.0 {
        if left.is_sign_negative() && right.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else {
        libm::fmax(left, right)
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
fn fmin_value(left: f64, right: f64) -> f64 {
    if left == 0.0 && right == 0.0 {
        // See fmax_value: the verified x86_64 GNU/Linux platform wrapper
        // preserves the first operand for equal signed zeros.
        left
    } else {
        libm::fmin(left, right)
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
fn fmin_value(left: f64, right: f64) -> f64 {
    if left == 0.0 && right == 0.0 {
        if left.is_sign_negative() || right.is_sign_negative() {
            -0.0
        } else {
            0.0
        }
    } else {
        libm::fmin(left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::{MathError, MathResult, evaluate};

    #[test]
    fn evaluates_scalar_and_tuple_math_at_the_public_seam() {
        assert_eq!(
            evaluate("fma", &[2.0, 3.0, 4.0]),
            Ok(MathResult::Scalar(10.0))
        );
        assert_eq!(evaluate("frexp", &[8.0]), Ok(MathResult::Pair([0.5, 4.0])));
        assert_eq!(evaluate("modf", &[3.5]), Ok(MathResult::Pair([0.5, 3.0])));
    }

    #[test]
    fn preserves_fused_rounding_and_signed_zero() {
        let fused = evaluate("fma", &[1.0 + f64::EPSILON, 1.0 - f64::EPSILON, -1.0]);
        assert_eq!(
            fused,
            Ok(MathResult::Scalar(f64::from_bits(0xb970_0000_0000_0000)))
        );

        let signed_zero = evaluate("copysign", &[0.0, -1.0]);
        assert!(matches!(
            signed_zero,
            Ok(MathResult::Scalar(value)) if value.to_bits() == (-0.0f64).to_bits()
        ));
        let domain_result = evaluate("sqrt", &[-1.0]);
        assert!(matches!(
            domain_result,
            Ok(MathResult::Scalar(value)) if value.is_nan()
        ));
    }

    #[test]
    fn covers_safe_aliases_and_scale_validation() {
        assert_eq!(evaluate("drem", &[5.0, 2.0]), Ok(MathResult::Scalar(1.0)));
        #[cfg(not(target_os = "linux"))]
        assert_eq!(evaluate("gamma", &[5.0]), Ok(MathResult::Scalar(24.0)));
        #[cfg(target_os = "linux")]
        assert_eq!(evaluate("gamma", &[5.0]), evaluate("lgamma", &[5.0]));
        assert_eq!(
            evaluate("nexttoward", &[1.0, 2.0]),
            Ok(MathResult::Scalar(f64::from_bits(1.0f64.to_bits() + 1),))
        );
        assert_eq!(evaluate("scalb", &[2.0, 3.0]), Ok(MathResult::Scalar(16.0)));
        assert_eq!(evaluate("ldexp", &[2.0, 0.5]), Ok(MathResult::Scalar(2.0)));
        assert_eq!(
            evaluate("scalbln", &[2.0, 0.5]),
            Ok(MathResult::Scalar(2.0))
        );
        #[cfg(not(target_os = "linux"))]
        assert_eq!(evaluate("scalb", &[2.0, 0.5]), Ok(MathResult::Scalar(2.0)));
        #[cfg(target_os = "linux")]
        assert!(matches!(
            evaluate("scalb", &[2.0, 0.5]),
            Ok(MathResult::Scalar(value)) if value.is_nan()
        ));
        assert_eq!(
            evaluate("jn", &[1_025.0, 0.0]),
            Err(MathError::InvalidOrder {
                function: "jn".to_owned(),
            })
        );
    }

    #[test]
    fn every_inventory_function_has_a_safe_scalar_or_pair_implementation() {
        let cases = [
            ("acos", &[0.5][..]),
            ("acosh", &[2.0][..]),
            ("asin", &[0.5][..]),
            ("asinh", &[0.5][..]),
            ("atan", &[0.5][..]),
            ("atan2", &[0.0, 1.0][..]),
            ("atanh", &[0.5][..]),
            ("cbrt", &[8.0][..]),
            ("ceil", &[0.5][..]),
            ("copysign", &[2.0, -1.0][..]),
            ("cos", &[0.5][..]),
            ("cosh", &[0.5][..]),
            ("drem", &[5.0, 2.0][..]),
            ("erf", &[0.5][..]),
            ("erfc", &[0.5][..]),
            ("exp", &[0.5][..]),
            ("exp10", &[0.5][..]),
            ("exp2", &[0.5][..]),
            ("expm1", &[0.5][..]),
            ("fabs", &[-0.5][..]),
            ("fdim", &[5.0, 2.0][..]),
            ("fma", &[2.0, 3.0, 4.0][..]),
            ("fmax", &[2.0, 3.0][..]),
            ("fmin", &[2.0, 3.0][..]),
            ("fmod", &[5.0, 2.0][..]),
            ("floor", &[0.5][..]),
            ("frexp", &[8.0][..]),
            ("gamma", &[5.0][..]),
            ("hypot", &[3.0, 4.0][..]),
            ("j0", &[0.0][..]),
            ("j1", &[0.0][..]),
            ("jn", &[0.0, 0.0][..]),
            ("ldexp", &[1.0, 3.0][..]),
            ("lgamma", &[5.0][..]),
            ("log", &[2.0][..]),
            ("log10", &[2.0][..]),
            ("log1p", &[0.5][..]),
            ("log2", &[2.0][..]),
            ("logb", &[8.0][..]),
            ("modf", &[3.5][..]),
            ("nearbyint", &[0.5][..]),
            ("nextafter", &[1.0, 2.0][..]),
            ("nexttoward", &[1.0, 2.0][..]),
            ("pow", &[2.0, 3.0][..]),
            ("remainder", &[5.0, 2.0][..]),
            ("rint", &[0.5][..]),
            ("round", &[0.5][..]),
            ("scalb", &[2.0, 3.0][..]),
            ("scalbln", &[2.0, 3.0][..]),
            ("significand", &[8.0][..]),
            ("sin", &[0.5][..]),
            ("sinh", &[0.5][..]),
            ("sqrt", &[2.0][..]),
            ("tan", &[0.5][..]),
            ("tanh", &[0.5][..]),
            ("tgamma", &[5.0][..]),
            ("trunc", &[0.5][..]),
            ("y0", &[1.0][..]),
            ("y1", &[1.0][..]),
            ("yn", &[0.0, 1.0][..]),
        ];
        for (name, arguments) in cases {
            assert!(evaluate(name, arguments).is_ok(), "{name}");
        }
    }

    #[test]
    fn reports_unknown_names_and_wrong_value_arity() {
        assert_eq!(
            evaluate("not_math", &[1.0]),
            Err(MathError::UnknownFunction)
        );
        assert_eq!(
            evaluate("atan2", &[1.0]),
            Err(MathError::Arity {
                function: "atan2".to_owned(),
                expected: 2,
                actual: 1,
            })
        );
    }
}
