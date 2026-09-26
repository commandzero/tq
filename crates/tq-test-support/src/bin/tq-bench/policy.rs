//! Selection and sampling policy for bounded performance feedback.

use tq_test_support::benchmark::{BenchmarkCase, BenchmarkSampling, BenchmarkTool};

/// Full-corpus fast feedback is intentionally narrower than correctness coverage.
pub(super) const FAST_LARGE_CASES: &[&str] =
    &["benchmark.parse-discard", "benchmark.dead-sort-length"];
pub(super) const LARGE_JSON_ADAPTERS: &[&str] = &["jq-json", "tq-json", "yq-json"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Mode {
    Fast,
    Exhaustive,
}

impl Mode {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "fast" => Ok(Self::Fast),
            "exhaustive" => Ok(Self::Exhaustive),
            _ => Err(format!(
                "invalid mode: {value}; expected fast or exhaustive"
            )),
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Exhaustive => "exhaustive",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Sampling {
    Quick,
    Screen,
    Compare,
    Extended,
    Catalog,
}

impl Sampling {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "quick" => Ok(Self::Quick),
            "screen" => Ok(Self::Screen),
            "compare" => Ok(Self::Compare),
            "extended" => Ok(Self::Extended),
            "catalog" => Ok(Self::Catalog),
            _ => Err(format!(
                "invalid sampling: {value}; expected quick, screen, compare, extended, or catalog"
            )),
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Screen => "screen",
            Self::Compare => "compare",
            Self::Extended => "extended",
            Self::Catalog => "catalog",
        }
    }

    pub(super) fn apply(self, case: &mut BenchmarkCase) {
        let (warmups, samples) = match self {
            Self::Quick => (0, 1),
            Self::Screen => (1, 1),
            Self::Compare => (1, 3),
            Self::Extended => (1, 5),
            Self::Catalog => return,
        };
        case.sampling = BenchmarkSampling {
            warmups,
            small: samples,
            medium: samples,
            large: samples,
        };
    }
}

pub(super) fn select_adapters(
    cases: &mut [BenchmarkCase],
    selected: &[String],
    mode: Mode,
) -> Result<(), String> {
    for id in selected {
        if !cases
            .iter()
            .any(|case| case.adapters.iter().any(|adapter| &adapter.id == id))
        {
            return Err(format!("unknown benchmark adapter: {id}"));
        }
    }
    for case in cases {
        if !selected.is_empty() {
            case.adapters.retain(|adapter| {
                selected.contains(&adapter.id)
                    || adapter.id == case.output_contract.reference_adapter
            });
        }
        if mode == Mode::Fast {
            // Preserve reference and candidate feedback before expensive third-party adapters.
            case.adapters.sort_by_key(|adapter| match adapter.tool {
                BenchmarkTool::Jq => 0,
                BenchmarkTool::Tq => 1,
                BenchmarkTool::Yq => 2,
            });
        }
    }
    Ok(())
}
