//! jq-compatible decimal-literal hybrid numbers.

use std::{
    cmp::Ordering,
    fmt,
    str::FromStr,
    sync::{Arc, OnceLock},
};

use num_bigint::BigInt;
use num_traits::Zero as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;

/// Accepted numeric resource envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumberLimits {
    /// Significant coefficient digits.
    pub coefficient_digits: usize,
    /// Absolute decimal exponent.
    pub absolute_exponent: u64,
    /// Maximum plain-decimal expansion.
    pub plain_expansion_digits: usize,
    /// Maximum rendered numeric token bytes.
    pub rendered_bytes: usize,
}

impl Default for NumberLimits {
    fn default() -> Self {
        Self {
            coefficient_digits: 4096,
            absolute_exponent: 1_000_000,
            plain_expansion_digits: 4096,
            rendered_bytes: 8192,
        }
    }
}

/// Numeric admission or arithmetic error.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum NumberError {
    /// Invalid JSON decimal grammar.
    #[error("invalid finite JSON number")]
    Invalid,
    /// Coefficient exceeds the digit envelope.
    #[error("numeric coefficient exceeds {limit} digits")]
    CoefficientDigits {
        /// Configured maximum.
        limit: usize,
    },
    /// Exponent exceeds the absolute envelope.
    #[error("numeric exponent exceeds {limit}")]
    Exponent {
        /// Configured maximum.
        limit: u64,
    },
    /// Canonical token exceeds its output envelope.
    #[error("rendered numeric token exceeds {limit} bytes")]
    RenderedBytes {
        /// Configured maximum.
        limit: usize,
    },
    /// Conversion or arithmetic produced a non-finite value.
    #[error("non-finite numeric value is outside the tq value model")]
    NonFinite,
    /// Division or remainder by zero.
    #[error("cannot divide by zero")]
    DivisionByZero,
}

/// Finite binary64 value with optional exact decimal literal provenance.
#[derive(Clone, Debug)]
pub struct Number {
    binary64: OnceLock<f64>,
    literal: Option<Arc<str>>,
}

impl Number {
    /// Parses a finite JSON number under the approved MVP limits.
    ///
    /// # Errors
    ///
    /// Returns a grammar, range, or resource error for an inadmissible token.
    pub fn parse(source: &str) -> Result<Self, NumberError> {
        Self::parse_with_limits(source, NumberLimits::default())
    }

    /// Canonicalizes a finite JSON numeric literal without constructing a
    /// runtime number.
    ///
    /// # Errors
    ///
    /// Returns a grammar, range, or resource error for an inadmissible token.
    pub fn canonicalize_literal(source: &str) -> Result<String, NumberError> {
        Self::canonicalize_literal_with_limits(source, NumberLimits::default())
    }

    /// Validates a JSON numeric literal without retaining its canonical value.
    ///
    /// This applies the same grammar and resource envelope as [`Self::parse`]
    /// while avoiding canonical-string allocation for values a caller will
    /// discard.
    ///
    /// # Errors
    ///
    /// Returns a grammar, range, or resource error for an inadmissible token.
    pub fn validate_literal(source: &str) -> Result<(), NumberError> {
        validate_literal_with_limits(source, NumberLimits::default())
    }

    /// Parses with explicit numeric resource limits.
    ///
    /// # Errors
    ///
    /// Returns a grammar, range, or resource error for an inadmissible token.
    pub fn parse_with_limits(source: &str, limits: NumberLimits) -> Result<Self, NumberError> {
        let literal = Self::canonicalize_literal_with_limits(source, limits)?;
        Ok(Self {
            binary64: OnceLock::new(),
            literal: Some(literal.into()),
        })
    }

    /// Canonicalizes a finite JSON numeric literal under explicit limits
    /// without constructing a runtime number.
    ///
    /// # Errors
    ///
    /// Returns a grammar, range, or resource error for an inadmissible token.
    pub fn canonicalize_literal_with_limits(
        source: &str,
        limits: NumberLimits,
    ) -> Result<String, NumberError> {
        let parts = DecimalParts::parse(source)?;
        if parts.coefficient_digits > limits.coefficient_digits {
            return Err(NumberError::CoefficientDigits {
                limit: limits.coefficient_digits,
            });
        }
        if parts.exponent.unsigned_abs() > limits.absolute_exponent {
            return Err(NumberError::Exponent {
                limit: limits.absolute_exponent,
            });
        }
        parts.canonical(limits)
    }

    /// Constructs an arithmetic-domain number and rejects NaN/infinity.
    ///
    /// # Errors
    ///
    /// Returns [`NumberError::NonFinite`] for NaN or infinity.
    pub fn from_f64(value: f64) -> Result<Self, NumberError> {
        if !value.is_finite() {
            return Err(NumberError::NonFinite);
        }
        Ok(Self {
            binary64: OnceLock::from(value),
            literal: None,
        })
    }

    /// Returns the lazily consumed binary64 interpretation.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        *self.binary64.get_or_init(|| {
            let parsed = self
                .literal
                .as_deref()
                .and_then(|literal| literal.parse::<f64>().ok())
                .unwrap_or(0.0);
            if parsed.is_infinite() {
                parsed.signum() * f64::MAX
            } else {
                parsed
            }
        })
    }

    /// Exact canonical literal retained from input, if arithmetic has not invalidated it.
    #[must_use]
    pub fn exact_literal(&self) -> Option<&str> {
        self.literal.as_deref()
    }

    /// True when this number is integral and inside jq's exact index envelope.
    #[must_use]
    pub fn exact_index(&self) -> Option<i64> {
        const MAX: f64 = 9_007_199_254_740_991.0;
        let value = self.as_f64();
        if value.fract() == 0.0 && value.abs() <= MAX {
            #[allow(clippy::cast_possible_truncation)]
            Some(value as i64)
        } else {
            None
        }
    }

    /// Adds values in the jq binary64 arithmetic domain.
    ///
    /// # Errors
    ///
    /// Returns a numeric range error if a finite result cannot be represented.
    pub fn add(&self, right: &Self) -> Result<Self, NumberError> {
        Self::arithmetic(self.as_f64() + right.as_f64())
    }

    /// Subtracts values in the jq binary64 arithmetic domain.
    ///
    /// # Errors
    ///
    /// Returns a numeric range error if a finite result cannot be represented.
    pub fn subtract(&self, right: &Self) -> Result<Self, NumberError> {
        Self::arithmetic(self.as_f64() - right.as_f64())
    }

    /// Multiplies values in the jq binary64 arithmetic domain.
    ///
    /// # Errors
    ///
    /// Returns a numeric range error if a finite result cannot be represented.
    pub fn multiply(&self, right: &Self) -> Result<Self, NumberError> {
        Self::arithmetic(self.as_f64() * right.as_f64())
    }

    /// Divides values in the jq binary64 arithmetic domain.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero divisor or non-finite result.
    pub fn divide(&self, right: &Self) -> Result<Self, NumberError> {
        if right.as_f64() == 0.0 {
            return Err(NumberError::DivisionByZero);
        }
        Self::arithmetic(self.as_f64() / right.as_f64())
    }

    fn arithmetic(value: f64) -> Result<Self, NumberError> {
        if value.is_infinite() {
            return Self::from_f64(value.signum() * f64::MAX);
        }
        Self::from_f64(value)
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.as_f64() == 0.0 && other.as_f64() == 0.0 {
            return Ordering::Equal;
        }
        if let (Some(left), Some(right)) = (&self.literal, &other.literal) {
            if let (Ok(left), Ok(right)) = (DecimalParts::parse(left), DecimalParts::parse(right)) {
                return compare_exact(&left, &right);
            }
        }
        self.as_f64().total_cmp(&other.as_f64())
    }
}

struct LiteralEnvelope<'a> {
    negative: bool,
    integer: &'a [u8],
    fraction: &'a [u8],
    exponent: i64,
}

fn parse_literal_envelope(source: &str) -> Result<LiteralEnvelope<'_>, NumberError> {
    let bytes = source.as_bytes();
    let mut index = usize::from(bytes.first() == Some(&b'-'));
    let negative = index == 1;
    if index >= bytes.len() {
        return Err(NumberError::Invalid);
    }

    let integer_start = index;
    if bytes[index] == b'0' {
        index += 1;
        if bytes.get(index).is_some_and(u8::is_ascii_digit) {
            return Err(NumberError::Invalid);
        }
    } else if bytes[index].is_ascii_digit() && bytes[index] != b'0' {
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
    } else {
        return Err(NumberError::Invalid);
    }
    let integer_end = index;

    let mut fraction_start = index;
    let mut fraction_end = index;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        fraction_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        fraction_end = index;
        if fraction_start == fraction_end {
            return Err(NumberError::Invalid);
        }
    }

    let mut exponent = 0_i64;
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        let exponent_negative = bytes.get(index) == Some(&b'-');
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            exponent = exponent
                .saturating_mul(10)
                .saturating_add(i64::from(bytes[index] - b'0'));
            index += 1;
        }
        if start == index {
            return Err(NumberError::Invalid);
        }
        if exponent_negative {
            exponent = -exponent;
        }
    }
    if index != bytes.len() {
        return Err(NumberError::Invalid);
    }

    Ok(LiteralEnvelope {
        negative,
        integer: &bytes[integer_start..integer_end],
        fraction: &bytes[fraction_start..fraction_end],
        exponent,
    })
}

fn validate_literal_with_limits(source: &str, limits: NumberLimits) -> Result<(), NumberError> {
    let LiteralEnvelope {
        negative,
        integer,
        fraction,
        exponent,
    } = parse_literal_envelope(source)?;
    let total_digits = integer.len().saturating_add(fraction.len());
    let leading_zeroes = integer
        .iter()
        .chain(fraction)
        .take_while(|digit| **digit == b'0')
        .count();
    let coefficient_digits = total_digits.saturating_sub(leading_zeroes).max(1);
    if coefficient_digits > limits.coefficient_digits {
        return Err(NumberError::CoefficientDigits {
            limit: limits.coefficient_digits,
        });
    }
    if exponent.unsigned_abs() > limits.absolute_exponent {
        return Err(NumberError::Exponent {
            limit: limits.absolute_exponent,
        });
    }
    if leading_zeroes == total_digits {
        return Ok(());
    }

    let trailing_zeroes = fraction
        .iter()
        .rev()
        .chain(integer.iter().rev())
        .take_while(|digit| **digit == b'0')
        .count();
    let digits = total_digits
        .saturating_sub(leading_zeroes)
        .saturating_sub(trailing_zeroes);
    let fraction_digits = i64::try_from(fraction.len()).unwrap_or(i64::MAX);
    let scale = exponent
        .saturating_sub(fraction_digits)
        .saturating_add(i64::try_from(trailing_zeroes).unwrap_or(i64::MAX));
    let plain_length = if scale >= 0 {
        digits.saturating_add(usize::try_from(scale).unwrap_or(usize::MAX))
    } else {
        digits.max(usize::try_from(-scale).unwrap_or(usize::MAX))
    };
    let sign = usize::from(negative);
    let rendered = if plain_length <= limits.plain_expansion_digits {
        if scale >= 0 {
            sign.saturating_add(plain_length)
        } else {
            let point = i64::try_from(digits)
                .unwrap_or(i64::MAX)
                .saturating_add(scale);
            if point > 0 {
                sign.saturating_add(digits).saturating_add(1)
            } else {
                sign.saturating_add(2)
                    .saturating_add(usize::try_from(-point).unwrap_or(usize::MAX))
                    .saturating_add(digits)
            }
        }
    } else {
        let scientific_exponent = scale
            .saturating_add(i64::try_from(digits).unwrap_or(i64::MAX))
            .saturating_sub(1);
        let exponent_bytes = decimal_i64_bytes(scientific_exponent);
        if digits == 1 {
            sign.saturating_add(2).saturating_add(exponent_bytes)
        } else {
            sign.saturating_add(digits)
                .saturating_add(2)
                .saturating_add(exponent_bytes)
        }
    };
    if rendered > limits.rendered_bytes {
        return Err(NumberError::RenderedBytes {
            limit: limits.rendered_bytes,
        });
    }
    Ok(())
}

fn decimal_i64_bytes(value: i64) -> usize {
    let magnitude = value.unsigned_abs();
    let digits = if magnitude == 0 {
        1
    } else {
        usize::try_from(magnitude.ilog10()).unwrap_or(usize::MAX) + 1
    };
    digits.saturating_add(usize::from(value < 0))
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(literal) = &self.literal {
            return formatter.write_str(literal);
        }
        let binary64 = self.as_f64();
        if binary64 == 0.0 {
            return formatter.write_str("0");
        }
        write!(formatter, "{binary64}")
    }
}

impl FromStr for Number {
    type Err = NumberError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Self::parse(source)
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let number =
            serde_json::Number::from_str(&self.to_string()).map_err(serde::ser::Error::custom)?;
        number.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let number = serde_json::Number::deserialize(deserializer)?;
        Self::parse(&number.to_string()).map_err(de::Error::custom)
    }
}

#[derive(Clone, Debug)]
struct DecimalParts {
    negative: bool,
    digits: String,
    scale: i64,
    exponent: i64,
    coefficient_digits: usize,
}

impl DecimalParts {
    fn parse(source: &str) -> Result<Self, NumberError> {
        let bytes = source.as_bytes();
        let mut index = usize::from(bytes.first() == Some(&b'-'));
        let negative = index == 1;
        if index >= bytes.len() {
            return Err(NumberError::Invalid);
        }
        let integer_start = index;
        if bytes[index] == b'0' {
            index += 1;
            if bytes.get(index).is_some_and(u8::is_ascii_digit) {
                return Err(NumberError::Invalid);
            }
        } else if bytes[index].is_ascii_digit() && bytes[index] != b'0' {
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
        } else {
            return Err(NumberError::Invalid);
        }
        let integer_end = index;
        let mut fraction_start = index;
        let mut fraction_end = index;
        if bytes.get(index) == Some(&b'.') {
            index += 1;
            fraction_start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            fraction_end = index;
            if fraction_start == fraction_end {
                return Err(NumberError::Invalid);
            }
        }
        let mut exponent = 0_i64;
        if matches!(bytes.get(index), Some(b'e' | b'E')) {
            index += 1;
            let exponent_negative = bytes.get(index) == Some(&b'-');
            if matches!(bytes.get(index), Some(b'+' | b'-')) {
                index += 1;
            }
            let start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                exponent = exponent
                    .saturating_mul(10)
                    .saturating_add(i64::from(bytes[index] - b'0'));
                index += 1;
            }
            if start == index {
                return Err(NumberError::Invalid);
            }
            if exponent_negative {
                exponent = -exponent;
            }
        }
        if index != bytes.len() {
            return Err(NumberError::Invalid);
        }
        let mut digits = String::from(&source[integer_start..integer_end]);
        if fraction_end > fraction_start {
            digits.push_str(&source[fraction_start..fraction_end]);
        }
        let fraction_digits =
            i64::try_from(fraction_end.saturating_sub(fraction_start)).unwrap_or(i64::MAX);
        let coefficient_digits = digits.trim_start_matches('0').len().max(1);
        Ok(Self {
            negative,
            digits,
            scale: exponent.saturating_sub(fraction_digits),
            exponent,
            coefficient_digits,
        })
    }

    fn canonical(&self, limits: NumberLimits) -> Result<String, NumberError> {
        let trimmed = self.digits.trim_start_matches('0');
        if trimmed.is_empty() {
            return Ok("0".to_owned());
        }
        let mut digits = trimmed.trim_end_matches('0').to_owned();
        let removed = trimmed.len() - digits.len();
        let scale = self
            .scale
            .saturating_add(i64::try_from(removed).unwrap_or(i64::MAX));
        let sign = if self.negative { "-" } else { "" };
        let plain_length = if scale >= 0 {
            digits
                .len()
                .saturating_add(usize::try_from(scale).unwrap_or(usize::MAX))
        } else {
            digits
                .len()
                .max(usize::try_from(-scale).unwrap_or(usize::MAX))
        };
        let output = if plain_length <= limits.plain_expansion_digits {
            if scale >= 0 {
                digits.extend(std::iter::repeat_n(
                    '0',
                    usize::try_from(scale).unwrap_or(0),
                ));
                format!("{sign}{digits}")
            } else {
                let point = i64::try_from(digits.len()).unwrap_or(i64::MAX) + scale;
                if point > 0 {
                    let point = usize::try_from(point).unwrap_or(digits.len());
                    digits.insert(point, '.');
                    format!("{sign}{digits}")
                } else {
                    let zeroes = usize::try_from(-point).unwrap_or(usize::MAX);
                    format!("{sign}0.{}{digits}", "0".repeat(zeroes))
                }
            }
        } else {
            let scientific_exponent = scale
                .saturating_add(i64::try_from(digits.len()).unwrap_or(i64::MAX))
                .saturating_sub(1);
            let first = digits.remove(0);
            if digits.is_empty() {
                format!("{sign}{first}e{scientific_exponent}")
            } else {
                format!("{sign}{first}.{digits}e{scientific_exponent}")
            }
        };
        if output.len() > limits.rendered_bytes {
            return Err(NumberError::RenderedBytes {
                limit: limits.rendered_bytes,
            });
        }
        Ok(output)
    }
}

fn compare_exact(left: &DecimalParts, right: &DecimalParts) -> Ordering {
    let left_coefficient = BigInt::from_str(&left.digits).unwrap_or_else(|_| BigInt::zero());
    let right_coefficient = BigInt::from_str(&right.digits).unwrap_or_else(|_| BigInt::zero());
    match (left_coefficient.is_zero(), right_coefficient.is_zero()) {
        (true, true) => return Ordering::Equal,
        (true, false) => {
            return if right.negative {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        (false, true) => {
            return if left.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        (false, false) => {}
    }
    if left.negative != right.negative {
        return if left.negative {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    let left_magnitude = i64::try_from(left.digits.trim_start_matches('0').len())
        .unwrap_or(i64::MAX)
        .saturating_add(left.scale);
    let right_magnitude = i64::try_from(right.digits.trim_start_matches('0').len())
        .unwrap_or(i64::MAX)
        .saturating_add(right.scale);
    let magnitude = left_magnitude.cmp(&right_magnitude);
    let unsigned = if magnitude == Ordering::Equal {
        let common_scale = left.scale.min(right.scale);
        let left_power = u32::try_from(left.scale - common_scale).unwrap_or(u32::MAX);
        let right_power = u32::try_from(right.scale - common_scale).unwrap_or(u32::MAX);
        (left_coefficient * BigInt::from(10_u8).pow(left_power))
            .cmp(&(right_coefficient * BigInt::from(10_u8).pow(right_power)))
    } else {
        magnitude
    };
    if left.negative {
        unsigned.reverse()
    } else {
        unsigned
    }
}

#[cfg(test)]
mod tests {
    use super::{Number, NumberError, NumberLimits};

    #[test]
    fn preserves_and_normalizes_exact_literals() {
        assert_eq!(
            Number::parse("9007199254740993").unwrap().to_string(),
            "9007199254740993"
        );
        assert_eq!(Number::parse("-0.0e99").unwrap().to_string(), "0");
        assert_eq!(Number::parse("12.3400").unwrap().to_string(), "12.34");
    }

    #[test]
    fn arithmetic_invalidates_literal_and_clamps_overflow() {
        let one = Number::parse("1").unwrap();
        let third = one.divide(&Number::parse("3").unwrap()).unwrap();
        assert!(third.exact_literal().is_none());
        assert!(
            (Number::from_f64(f64::MAX)
                .unwrap()
                .multiply(&Number::parse("2").unwrap())
                .unwrap()
                .as_f64()
                - f64::MAX)
                .abs()
                < f64::EPSILON
        );
        assert_eq!(
            one.divide(&Number::parse("0").unwrap()),
            Err(NumberError::DivisionByZero)
        );
    }

    #[test]
    fn enforces_digit_and_exponent_limits() {
        let limits = NumberLimits {
            coefficient_digits: 2,
            absolute_exponent: 2,
            ..NumberLimits::default()
        };
        assert!(matches!(
            Number::parse_with_limits("123", limits),
            Err(NumberError::CoefficientDigits { .. })
        ));
        assert!(matches!(
            Number::parse_with_limits("1e3", limits),
            Err(NumberError::Exponent { .. })
        ));
    }

    #[test]
    fn validation_only_literals_match_retained_number_admission() {
        for literal in [
            "0",
            "-0.0e99",
            "12.3400",
            "9007199254740993",
            "1e1000000",
            "1e-1000000",
            "01",
            "1.",
            "1e",
            "1e1000001",
        ] {
            assert_eq!(
                Number::validate_literal(literal),
                Number::parse(literal).map(|_| ()),
                "literal {literal}"
            );
        }

        let oversized = "1".repeat(NumberLimits::default().coefficient_digits + 1);
        assert_eq!(
            Number::validate_literal(&oversized),
            Number::parse(&oversized).map(|_| ())
        );
    }

    #[test]
    fn exact_domain_orders_beyond_binary64_and_handles_index_boundaries() {
        let lower = Number::parse("9007199254740992").unwrap();
        let higher = Number::parse("9007199254740993").unwrap();
        assert!(lower < higher);
        assert_ne!(lower, higher);
        assert_eq!(
            Number::parse("9007199254740991").unwrap().exact_index(),
            Some(9_007_199_254_740_991)
        );
        assert_eq!(higher.exact_index(), None);
        assert!(Number::parse("-0.01").unwrap() < Number::parse("0").unwrap());
        assert!(Number::parse("0").unwrap() < Number::parse("0.01").unwrap());
    }

    #[test]
    fn negative_zero_and_large_exponents_render_canonically() {
        let negative_zero = Number::parse("-0e100").unwrap();
        assert_eq!(negative_zero, Number::from_f64(-0.0).unwrap());
        assert_eq!(negative_zero.to_string(), "0");
        assert_eq!(Number::parse("1e1000000").unwrap().to_string(), "1e1000000");
        assert_eq!(
            Number::parse("1e-1000000").unwrap().to_string(),
            "1e-1000000"
        );
    }
}
