//! Loss-preserving benchmark run metadata for authored Markdown pages.

use yaml_serde::Value;

/// Merges one measured campaign into a page's YAML frontmatter.
///
/// Existing YAML keys and the authored Markdown body are retained. Malformed
/// frontmatter is rejected rather than overwritten.
/// # Errors
///
/// Returns an error if existing frontmatter is malformed, unterminated, or
/// not a mapping; if `benchmark_runs` is not a sequence; or if serialization
/// fails.
pub fn merge_benchmark_run(
    source: &str,
    campaign_id: &str,
    binaries: Value,
) -> Result<String, String> {
    let (frontmatter, body) = split_frontmatter(source)?;
    let mut frontmatter = frontmatter.unwrap_or_else(|| Value::Mapping(yaml_serde::Mapping::new()));
    let Value::Mapping(metadata) = &mut frontmatter else {
        return Err("benchmark frontmatter must be a YAML mapping".to_owned());
    };

    let runs_key = Value::String("benchmark_runs".to_owned());
    let mut runs = match metadata.remove(&runs_key) {
        None => Vec::new(),
        Some(Value::Sequence(runs)) => runs,
        Some(_) => return Err("benchmark_runs frontmatter field must be a sequence".to_owned()),
    };
    let mut run = yaml_serde::Mapping::new();
    run.insert(
        Value::String("campaign_id".to_owned()),
        Value::String(campaign_id.to_owned()),
    );
    run.insert(Value::String("binaries".to_owned()), binaries);
    if campaign_id.starts_with("not-recorded:") {
        run.insert(
            Value::String("identity_status".to_owned()),
            Value::String("not-recorded".to_owned()),
        );
    }
    if let Some(existing) = runs
        .iter_mut()
        .find(|existing| existing.get("campaign_id").and_then(Value::as_str) == Some(campaign_id))
    {
        if let Value::Mapping(existing_mapping) = existing {
            for (key, value) in run {
                existing_mapping.insert(key, value);
            }
        } else {
            *existing = Value::Mapping(run);
        }
    } else {
        runs.push(Value::Mapping(run));
    }
    metadata.insert(runs_key, Value::Sequence(runs));

    let encoded = yaml_serde::to_string(&frontmatter)
        .map_err(|error| format!("cannot serialize benchmark frontmatter: {error}"))?;
    let encoded = encoded.strip_suffix('\n').unwrap_or(&encoded);
    Ok(format!("---\n{encoded}\n---\n{body}"))
}

fn split_frontmatter(source: &str) -> Result<(Option<Value>, &str), String> {
    if !source.starts_with("---") {
        return Ok((None, source));
    }
    if !source.starts_with("---\n") && !source.starts_with("---\r\n") {
        return Err("malformed benchmark YAML frontmatter opening delimiter".to_owned());
    }
    let start = if source.starts_with("---\r\n") { 5 } else { 4 };
    let rest = &source[start..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let content = line
            .strip_suffix('\n')
            .unwrap_or(line)
            .strip_suffix('\r')
            .unwrap_or(line.strip_suffix('\n').unwrap_or(line));
        let next = offset + line.len();
        if content == "---" {
            let yaml = &rest[..offset];
            let body = &rest[next..];
            let value = yaml_serde::from_str(yaml)
                .map_err(|error| format!("malformed benchmark YAML frontmatter: {error}"))?;
            return Ok((Some(value), body));
        }
        offset = next;
    }
    Err("unterminated benchmark YAML frontmatter".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Value, merge_benchmark_run};

    fn binaries(version: &str) -> Value {
        yaml_serde::from_str(&format!("{{tq: {{version: '{version}', sha256: digest}}}}"))
            .expect("valid tool metadata")
    }

    #[test]
    fn replacing_campaign_metadata_keeps_one_current_identity() {
        let source = "---\ntitle: authored\nbenchmark_runs:\n  - campaign_id: run-1\n    captured_at: 2026-01-02T03:04:05Z\n    source_role: release\n    binaries: {tq: {version: old, sha256: old}}\n---\nAuthored body.\n";
        let rendered = merge_benchmark_run(source, "run-1", binaries("new")).unwrap();
        let frontmatter = rendered.split("\n---\n").next().unwrap();
        let value: Value = yaml_serde::from_str(frontmatter.trim_start_matches("---\n")).unwrap();
        assert_eq!(value["benchmark_runs"].as_sequence().unwrap().len(), 1);
        assert_eq!(
            value["benchmark_runs"][0]["binaries"]["tq"]["version"].as_str(),
            Some("new")
        );
        assert_eq!(
            value["benchmark_runs"][0]["captured_at"].as_str(),
            Some("2026-01-02T03:04:05Z")
        );
        assert_eq!(
            value["benchmark_runs"][0]["source_role"].as_str(),
            Some("release")
        );
        assert!(rendered.ends_with("Authored body.\n"));
    }

    #[test]
    fn distinct_campaign_tool_identities_are_preserved() {
        let first = merge_benchmark_run("# Title\n", "run-1", binaries("1.0")).unwrap();
        let second = merge_benchmark_run(&first, "run-2", binaries("2.0")).unwrap();
        let yaml = second
            .split("\n---\n")
            .next()
            .unwrap()
            .trim_start_matches("---\n");
        let value: Value = yaml_serde::from_str(yaml).unwrap();
        let runs = value["benchmark_runs"].as_sequence().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0]["binaries"]["tq"]["version"].as_str(), Some("1.0"));
        assert_eq!(runs[1]["binaries"]["tq"]["version"].as_str(), Some("2.0"));
    }
}
