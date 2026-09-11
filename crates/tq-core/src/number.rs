//! jq-compatible decimal-literal hybrid numbers.

use std::{
    cmp::Ordering,
    fmt,
    str::FromStr,
    sync::{Arc, OnceLock},
};

use num_bigint::BigInt;
use num_traits::ToPrimitive;
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
            // jq's decimal backend accepts exponents well beyond the range
            // useful to binary64. Keep this bounded, but do not reject the
            // manual's 1E1234567890 identity witness merely because it never
            // needs a plain-decimal expansion.
            absolute_exponent: 2_000_000_000,
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
    /// A finite-only admission path received a non-finite value.
    #[error("non-finite numeric value is not accepted by finite input admission")]
    NonFinite,
    /// Division or remainder by zero.
    #[error("cannot divide by zero")]
    DivisionByZero,
}

/// Binary64 runtime value with optional exact finite decimal literal provenance.
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
        if let Some(literal) = canonical_plain_integer(source, limits)? {
            return Ok(literal);
        }
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

    /// Constructs a finite number for input admission and rejects NaN/infinity.
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

    /// Constructs a computed number, including NaN and infinities.
    ///
    /// Native input admission remains finite through [`Self::from_f64`] and
    /// [`Self::parse`]. Non-finite runtime values keep their numeric identity
    /// until serialization projects NaN to null and infinity to finite bounds.
    #[must_use]
    pub fn from_runtime_f64(value: f64) -> Self {
        Self {
            binary64: OnceLock::from(value),
            literal: None,
        }
    }

    /// Negates a number while retaining exact decimal literal provenance.
    pub(crate) fn negate(&self) -> Self {
        if let Some(literal) = &self.literal {
            let is_zero = DecimalParts::parse(literal)
                .is_ok_and(|parts| parts.digits.trim_matches('0').is_empty());
            let literal = if is_zero {
                literal.strip_prefix('-').unwrap_or(literal).to_owned()
            } else if let Some(positive) = literal.strip_prefix('-') {
                positive.to_owned()
            } else {
                format!("-{literal}")
            };
            return Self {
                binary64: OnceLock::new(),
                literal: Some(literal.into()),
            };
        }
        Self::from_runtime_f64(-self.as_f64())
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

    /// Compares the number's mathematical value with zero without narrowing
    /// an admitted decimal literal through binary64 first.
    pub(crate) fn is_less_than_zero(&self) -> bool {
        if let Some(literal) = &self.literal
            && let Ok(parts) = DecimalParts::parse(literal)
        {
            return compare_exact(&parts, &DecimalParts::zero(false)) == Ordering::Less;
        }
        self.as_f64() < 0.0
    }

    /// Exact canonical literal retained from input, if arithmetic has not invalidated it.
    #[must_use]
    pub fn exact_literal(&self) -> Option<&str> {
        self.literal.as_deref()
    }

    /// Returns a lossless representation for mathematical result
    /// normalization. Unlike [`Self::to_string`], this intentionally removes
    /// insignificant decimal scale (so `1`, `1.0`, and `100e-2` compare as
    /// one value) without converting through binary64 or expanding exponents.
    #[must_use]
    pub fn canonical_numeric(&self) -> String {
        let Some(literal) = &self.literal else {
            if self.as_f64() == 0.0 {
                return "0".to_owned();
            }
            return self.to_string();
        };
        let Ok(parts) = DecimalParts::parse(literal) else {
            return self.to_string();
        };
        canonical_numeric_parts(&parts)
    }

    /// Canonicalizes a finite decimal for semantic comparison without
    /// projecting through the runtime binary64 value model.
    ///
    /// This is intentionally separate from [`Self::parse`]: very large or
    /// small exponents must remain distinct during compatibility comparison,
    /// even though runtime JSON projection may clamp them.
    ///
    /// # Errors
    ///
    /// Returns [`NumberError::Invalid`] when `source` is not a JSON decimal.
    pub fn canonicalize_literal_numeric(source: &str) -> Result<String, NumberError> {
        canonicalize_unbounded_numeric_literal(source)
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
        Ok(Self::arithmetic(self.as_f64() + right.as_f64()))
    }

    /// Subtracts values in the jq binary64 arithmetic domain.
    ///
    /// # Errors
    ///
    /// Returns a numeric range error if a finite result cannot be represented.
    pub fn subtract(&self, right: &Self) -> Result<Self, NumberError> {
        Ok(Self::arithmetic(self.as_f64() - right.as_f64()))
    }

    /// Multiplies values in the jq binary64 arithmetic domain.
    ///
    /// # Errors
    ///
    /// Returns a numeric range error if a finite result cannot be represented.
    pub fn multiply(&self, right: &Self) -> Result<Self, NumberError> {
        Ok(Self::arithmetic(self.as_f64() * right.as_f64()))
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
        Ok(Self::arithmetic(self.as_f64() / right.as_f64()))
    }

    fn arithmetic(value: f64) -> Self {
        Self::from_runtime_f64(value)
    }

    fn compare(&self, other: &Self) -> Ordering {
        if let (Some(left), Some(right)) = (&self.literal, &other.literal)
            && let (Ok(left), Ok(right)) = (DecimalParts::parse(left), DecimalParts::parse(right))
        {
            return compare_exact(&left, &right);
        }
        if let Some(left) = &self.literal
            && other.as_f64() == 0.0
            && let Ok(left) = DecimalParts::parse(left)
        {
            return compare_exact(&left, &DecimalParts::zero(false));
        }
        if let Some(right) = &other.literal
            && self.as_f64() == 0.0
            && let Ok(right) = DecimalParts::parse(right)
        {
            return compare_exact(&DecimalParts::zero(false), &right);
        }
        if self.as_f64() == 0.0 && other.as_f64() == 0.0 {
            return Ordering::Equal;
        }
        self.as_f64().total_cmp(&other.as_f64())
    }
}

fn canonical_numeric_parts(parts: &DecimalParts) -> String {
    let digits = parts.digits.trim_start_matches('0');
    if digits.is_empty() {
        return "0".to_owned();
    }
    let trimmed = digits.trim_end_matches('0');
    let removed = digits.len().saturating_sub(trimmed.len());
    let scale = parts
        .scale
        .saturating_add(i64::try_from(removed).unwrap_or(i64::MAX));
    let sign = if parts.negative { "-" } else { "" };
    let adjusted = scale
        .saturating_add(i64::try_from(trimmed.len()).unwrap_or(i64::MAX))
        .saturating_sub(1);
    let plain_length = if scale >= 0 {
        trimmed
            .len()
            .saturating_add(usize::try_from(scale).unwrap_or(usize::MAX))
    } else {
        let point = i64::try_from(trimmed.len())
            .unwrap_or(i64::MAX)
            .saturating_add(scale);
        if point > 0 {
            trimmed.len().saturating_add(1)
        } else {
            2usize
                .saturating_add(usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX))
                .saturating_add(trimmed.len())
        }
    };
    if adjusted >= -6 && plain_length <= NumberLimits::default().plain_expansion_digits {
        if scale == 0 {
            return format!("{sign}{trimmed}");
        }
        if scale > 0 {
            return format!(
                "{sign}{trimmed}{}",
                "0".repeat(usize::try_from(scale).unwrap_or(usize::MAX))
            );
        }
        let point = i64::try_from(trimmed.len())
            .unwrap_or(i64::MAX)
            .saturating_add(scale);
        if point > 0 {
            let point = usize::try_from(point).unwrap_or(trimmed.len());
            return format!("{sign}{}.{}", &trimmed[..point], &trimmed[point..]);
        }
        let zeroes = usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX);
        return format!("{sign}0.{}{trimmed}", "0".repeat(zeroes));
    }
    let first = trimmed.as_bytes()[0] as char;
    let rest = &trimmed[1..];
    let exponent_sign = if adjusted >= 0 { "+" } else { "" };
    if rest.is_empty() {
        format!("{sign}{first}E{exponent_sign}{adjusted}")
    } else {
        format!("{sign}{first}.{rest}E{exponent_sign}{adjusted}")
    }
}

/// Canonicalizes a decimal for semantic comparison without narrowing its
/// exponent to the runtime i64 envelope. Saturating such an exponent would
/// make adjacent, distinct literals compare equal.
#[allow(
    clippy::too_many_lines,
    reason = "lossless decimal normalization keeps grammar and arbitrary-exponent handling together"
)]
fn canonicalize_unbounded_numeric_literal(source: &str) -> Result<String, NumberError> {
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
    } else if bytes[index].is_ascii_digit() {
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

    let mut exponent = BigInt::from(0);
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        let exponent_negative = bytes.get(index) == Some(&b'-');
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if start == index {
            return Err(NumberError::Invalid);
        }
        exponent = BigInt::parse_bytes(&bytes[start..index], 10).ok_or(NumberError::Invalid)?;
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
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Ok("0".to_owned());
    }
    let trimmed = digits.trim_end_matches('0');
    let removed = i64::try_from(digits.len().saturating_sub(trimmed.len()))
        .map_err(|_| NumberError::Invalid)?;
    let fraction = i64::try_from(fraction_end.saturating_sub(fraction_start))
        .map_err(|_| NumberError::Invalid)?;
    let scale = exponent + BigInt::from(removed.saturating_sub(fraction));
    let digit_count = i64::try_from(trimmed.len()).map_err(|_| NumberError::Invalid)?;
    let adjusted = &scale + BigInt::from(digit_count.saturating_sub(1));
    let sign = if negative { "-" } else { "" };

    if let Some(scale) = scale.to_i64()
        && let Some(adjusted) = adjusted.to_i64()
    {
        let plain_length = if scale >= 0 {
            trimmed
                .len()
                .saturating_add(usize::try_from(scale).unwrap_or(usize::MAX))
        } else {
            let point = digit_count.saturating_add(scale);
            if point > 0 {
                trimmed.len().saturating_add(1)
            } else {
                2usize
                    .saturating_add(usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX))
                    .saturating_add(trimmed.len())
            }
        };
        if adjusted >= -6 && plain_length <= NumberLimits::default().plain_expansion_digits {
            if scale == 0 {
                return Ok(format!("{sign}{trimmed}"));
            }
            if scale > 0 {
                return Ok(format!(
                    "{sign}{trimmed}{}",
                    "0".repeat(usize::try_from(scale).unwrap_or(usize::MAX))
                ));
            }
            let point = digit_count.saturating_add(scale);
            if point > 0 {
                let point = usize::try_from(point).unwrap_or(trimmed.len());
                return Ok(format!("{sign}{}.{}", &trimmed[..point], &trimmed[point..]));
            }
            let zeroes = usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX);
            return Ok(format!("{sign}0.{}{trimmed}", "0".repeat(zeroes)));
        }
    }

    let first = trimmed.as_bytes()[0] as char;
    let rest = &trimmed[1..];
    let exponent = adjusted.to_string();
    if rest.is_empty() {
        Ok(format!("{sign}{first}E{exponent}"))
    } else {
        Ok(format!("{sign}{first}.{rest}E{exponent}"))
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
                    .saturating_add(usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX))
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
        if binary64.is_nan() {
            return formatter.write_str("null");
        }
        if binary64.is_infinite() {
            return formatter.write_str(&runtime_f64_string(binary64.signum() * f64::MAX));
        }
        if binary64 == 0.0 {
            return formatter.write_str(if binary64.is_sign_negative() {
                "-0"
            } else {
                "0"
            });
        }
        formatter.write_str(&runtime_f64_string(binary64))
    }
}

fn runtime_f64_string(value: f64) -> String {
    let mut rendered = serde_json::Number::from_f64(value)
        .expect("finite runtime values are valid JSON numbers")
        .to_string();
    if let Some(dot) = rendered.find('.')
        && rendered[dot + 1..] == *"0"
    {
        rendered.truncate(dot);
    }
    let Some(exponent) = rendered.find(['e', 'E']) else {
        return rendered;
    };
    let marker = exponent + 1;
    let exponent_digits = rendered[marker..].to_owned();
    let (sign, digits) = match exponent_digits.as_bytes().first() {
        Some(b'+' | b'-') => (&exponent_digits[..1], &exponent_digits[1..]),
        _ => ("+", exponent_digits.as_str()),
    };
    let digits = if sign == "-" && digits.len() == 1 {
        format!("0{digits}")
    } else {
        digits.to_owned()
    };
    rendered.truncate(marker);
    rendered.push_str(sign);
    rendered.push_str(&digits);
    rendered
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
        if self.literal.is_none() && self.as_f64().is_nan() {
            return serializer.serialize_unit();
        }
        let spelling = self.to_string();
        let number = serde_json::Number::from_str(&spelling).map_err(serde::ser::Error::custom)?;
        if number.as_str() != spelling {
            let raw = serde_json::value::RawValue::from_string(spelling)
                .map_err(serde::ser::Error::custom)?;
            return raw.serialize(serializer);
        }
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
    fn zero(negative: bool) -> Self {
        Self {
            negative,
            digits: "0".to_owned(),
            scale: 0,
            exponent: 0,
            coefficient_digits: 1,
        }
    }

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
        let digits = self.digits.trim_start_matches('0');
        let sign = if self.negative { "-" } else { "" };
        if digits.is_empty() {
            let output = format!(
                "{sign}{}",
                render_zero(self.scale, limits.plain_expansion_digits)
            );
            if output.len() > limits.rendered_bytes {
                return Err(NumberError::RenderedBytes {
                    limit: limits.rendered_bytes,
                });
            }
            return Ok(output);
        }
        let digit_count = i64::try_from(digits.len()).unwrap_or(i64::MAX);
        let adjusted_exponent = self.scale.saturating_add(digit_count).saturating_sub(1);
        // decNumber's jq context has a nine-digit adjusted-exponent envelope.
        // Values beyond it become infinities and jq projects those through
        // the largest finite binary64 value rather than emitting an
        // unbounded exponent token.
        if adjusted_exponent > 999_999_999 {
            let output = if self.negative {
                "-1.7976931348623157e+308".to_owned()
            } else {
                "1.7976931348623157e+308".to_owned()
            };
            if output.len() > limits.rendered_bytes {
                return Err(NumberError::RenderedBytes {
                    limit: limits.rendered_bytes,
                });
            }
            return Ok(output);
        }
        let plain_length = if self.scale >= 0 {
            digits
                .len()
                .saturating_add(usize::try_from(self.scale).unwrap_or(usize::MAX))
        } else {
            let point = digit_count.saturating_add(self.scale);
            if point > 0 {
                digits.len().saturating_add(1)
            } else {
                2usize
                    .saturating_add(usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX))
                    .saturating_add(digits.len())
            }
        };
        let output = if self.scale <= 0
            && adjusted_exponent >= -6
            && plain_length <= limits.plain_expansion_digits
        {
            if self.scale == 0 {
                format!("{sign}{digits}")
            } else {
                let point = digit_count.saturating_add(self.scale);
                if point > 0 {
                    let point = usize::try_from(point).unwrap_or(digits.len());
                    format!("{sign}{}.{}", &digits[..point], &digits[point..])
                } else {
                    let zeroes = usize::try_from(point.unsigned_abs()).unwrap_or(usize::MAX);
                    format!("{sign}0.{}{digits}", "0".repeat(zeroes))
                }
            }
        } else {
            let first = digits.as_bytes()[0] as char;
            let rest = &digits[1..];
            let exponent_sign = if adjusted_exponent >= 0 { "+" } else { "" };
            if rest.is_empty() {
                format!("{sign}{first}E{exponent_sign}{adjusted_exponent}")
            } else {
                format!("{sign}{first}.{rest}E{exponent_sign}{adjusted_exponent}")
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

/// Returns an already-canonical plain integer without allocating an
/// intermediate coefficient buffer. The decimal-parts parser remains the general path for
/// fractions and exponent notation, whose spelling needs normalization.
fn canonical_plain_integer(
    source: &str,
    limits: NumberLimits,
) -> Result<Option<String>, NumberError> {
    let bytes = source.as_bytes();
    let mut index = usize::from(bytes.first() == Some(&b'-'));
    if index >= bytes.len() {
        return Ok(None);
    }
    let integer_start = index;
    if bytes[index] == b'0' {
        index += 1;
        if bytes.get(index).is_some_and(u8::is_ascii_digit) {
            return Ok(None);
        }
    } else if bytes[index].is_ascii_digit() {
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
    } else {
        return Ok(None);
    }
    if index != bytes.len() {
        return Ok(None);
    }
    let coefficient_digits = bytes[integer_start..]
        .iter()
        .skip_while(|digit| **digit == b'0')
        .count()
        .max(1);
    if coefficient_digits > limits.coefficient_digits {
        return Err(NumberError::CoefficientDigits {
            limit: limits.coefficient_digits,
        });
    }
    // DecimalParts::canonical switches to scientific notation when a
    // nonzero plain integer exceeds this expansion budget. Keep that
    // representation and its rendered-byte check on the general path.
    if bytes[integer_start..].iter().any(|digit| *digit != b'0')
        && coefficient_digits > limits.plain_expansion_digits
    {
        return Ok(None);
    }
    if source.len() > limits.rendered_bytes {
        return Err(NumberError::RenderedBytes {
            limit: limits.rendered_bytes,
        });
    }
    Ok(Some(source.to_owned()))
}

fn render_zero(scale: i64, plain_expansion_digits: usize) -> String {
    if scale == 0 {
        return "0".to_owned();
    }
    if scale > 0 {
        return format!("0E+{scale}");
    }
    let fractional = scale.unsigned_abs();
    if fractional <= 6 && fractional <= plain_expansion_digits as u64 {
        return format!("0.{}", "0".repeat(usize::try_from(fractional).unwrap_or(0)));
    }
    format!("0E{scale}")
}

fn compare_exact(left: &DecimalParts, right: &DecimalParts) -> Ordering {
    let left_digits = left.digits.trim_start_matches('0');
    let right_digits = right.digits.trim_start_matches('0');
    match (left_digits.is_empty(), right_digits.is_empty()) {
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
    let left_magnitude = i64::try_from(left_digits.len())
        .unwrap_or(i64::MAX)
        .saturating_add(left.scale);
    let right_magnitude = i64::try_from(right_digits.len())
        .unwrap_or(i64::MAX)
        .saturating_add(right.scale);
    let magnitude = left_magnitude.cmp(&right_magnitude);
    let unsigned = if magnitude == Ordering::Equal {
        // Equal adjusted exponents align the most significant digit. Pad the
        // shorter coefficient with implicit zeroes instead of expanding
        // powers of ten, which keeps huge exponents bounded.
        let length = left_digits.len().max(right_digits.len());
        (0..length)
            .map(|index| {
                (
                    left_digits.as_bytes().get(index).copied().unwrap_or(b'0'),
                    right_digits.as_bytes().get(index).copied().unwrap_or(b'0'),
                )
            })
            .find_map(
                |(left_digit, right_digit)| match left_digit.cmp(&right_digit) {
                    Ordering::Equal => None,
                    ordering => Some(ordering),
                },
            )
            .unwrap_or(Ordering::Equal)
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
    use super::{DecimalParts, Number, NumberError, NumberLimits};

    fn general_render(source: &str, limits: NumberLimits) -> Result<String, NumberError> {
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

    #[test]
    fn preserves_jq_literal_representation() {
        assert_eq!(
            Number::parse("9007199254740993").unwrap().to_string(),
            "9007199254740993"
        );
        assert_eq!(Number::parse("-0.0e99").unwrap().to_string(), "-0E+98");
        assert_eq!(Number::parse("12.3400").unwrap().to_string(), "12.3400");
        assert_eq!(Number::parse("1.000").unwrap().to_string(), "1.000");
        assert_eq!(Number::parse("100e-2").unwrap().to_string(), "1.00");
        assert_eq!(Number::parse("1e2").unwrap().to_string(), "1E+2");
    }

    #[test]
    fn plain_integer_fast_path_matches_general_limits() {
        let limits = [
            NumberLimits::default(),
            NumberLimits {
                coefficient_digits: 2,
                ..NumberLimits::default()
            },
            NumberLimits {
                absolute_exponent: 0,
                ..NumberLimits::default()
            },
            NumberLimits {
                plain_expansion_digits: 2,
                ..NumberLimits::default()
            },
            NumberLimits {
                rendered_bytes: 2,
                ..NumberLimits::default()
            },
            NumberLimits {
                plain_expansion_digits: 2,
                rendered_bytes: 3,
                ..NumberLimits::default()
            },
            NumberLimits {
                plain_expansion_digits: 0,
                rendered_bytes: 2,
                ..NumberLimits::default()
            },
        ];
        for source in ["0", "-0", "12", "-123", "123", "01", "1.", "1e", "-"] {
            for limits in limits {
                let expected = general_render(source, limits);
                assert_eq!(
                    Number::canonicalize_literal_with_limits(source, limits),
                    expected,
                    "canonicalization drift for {source:?} and {limits:?}"
                );
                assert_eq!(
                    Number::parse_with_limits(source, limits).map(|number| number.to_string()),
                    expected,
                    "admission drift for {source:?} and {limits:?}"
                );
            }
        }
    }

    #[test]
    fn arithmetic_invalidates_literal_and_keeps_runtime_overflow() {
        let one = Number::parse("1").unwrap();
        let third = one.divide(&Number::parse("3").unwrap()).unwrap();
        assert!(third.exact_literal().is_none());
        assert!(
            Number::from_f64(f64::MAX)
                .unwrap()
                .multiply(&Number::parse("2").unwrap())
                .unwrap()
                .as_f64()
                .is_infinite()
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
        assert_eq!(negative_zero.to_string(), "-0E+100");
        assert_eq!(
            Number::parse("1e1000000").unwrap().to_string(),
            "1E+1000000"
        );
        assert_eq!(
            Number::parse("1e-1000000").unwrap().to_string(),
            "1E-1000000"
        );
    }

    #[test]
    fn underflowing_literals_remain_distinct_from_zero_and_each_other() {
        let tiny = Number::parse("1e-400").unwrap();
        let tinier = Number::parse("2e-400").unwrap();
        let zero = Number::parse("0").unwrap();
        assert!(tiny > zero);
        assert!(tinier > tiny);
        assert_ne!(tiny, zero);
    }

    #[test]
    fn large_exponent_identity_does_not_expand() {
        let number = Number::parse("1E1234567890").unwrap();
        assert_eq!(number.to_string(), "1.7976931348623157e+308");
        assert!(number.as_f64().is_finite());
    }
}
