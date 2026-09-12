//! The semantic completeness ledger must reference executable catalog cases.

use std::collections::BTreeSet;
use std::path::Path;
use tq_test_support::compatibility::{CaseStatus, load_catalog};

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn skip_quoted(query: &[u8], mut offset: usize) -> usize {
    debug_assert_eq!(query[offset], b'"');
    offset += 1;
    while offset < query.len() {
        match query[offset] {
            b'\\' => offset = offset.saturating_add(2),
            b'"' => return offset + 1,
            _ => offset += 1,
        }
    }
    query.len()
}

fn skip_comment(query: &[u8], mut offset: usize) -> usize {
    debug_assert_eq!(query[offset], b'#');
    while offset < query.len() && query[offset] != b'\n' {
        offset += 1;
    }
    offset
}

fn matching_parenthesis(query: &[u8], opening: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut offset = opening;
    while offset < query.len() {
        if query[offset] == b'"' {
            offset = skip_quoted(query, offset);
            continue;
        }
        if query[offset] == b'#' {
            offset = skip_comment(query, offset);
            continue;
        }
        match query[offset] {
            b'(' => depth += 1,
            b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
        offset += 1;
    }
    None
}

fn call_argument_count(query: &[u8], opening: usize, closing: usize) -> usize {
    let mut nested = 0usize;
    let mut separators = 0usize;
    let mut offset = opening + 1;
    while offset < closing {
        if query[offset] == b'"' {
            offset = skip_quoted(query, offset);
            continue;
        }
        if query[offset] == b'#' {
            offset = skip_comment(query, offset);
            continue;
        }
        match query[offset] {
            b'(' => nested += 1,
            b')' => nested = nested.saturating_sub(1),
            b';' if nested == 0 => separators += 1,
            _ => {}
        }
        offset += 1;
    }
    if query[opening + 1..closing]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        0
    } else {
        separators + 1
    }
}

/// Finds a real jq call, ignoring quoted strings and requiring its exact
/// semicolon-delimited argument count. This intentionally stays local to the
/// evidence gate: tq-core's AST is crate-private and exposing it solely for a
/// fixture validator would make the production API shallower.
fn query_calls_arity(query: &str, name: &str, arity: usize) -> bool {
    let query = query.as_bytes();
    let name = name.as_bytes();
    let mut offset = 0usize;
    while offset + name.len() <= query.len() {
        if query[offset] == b'"' {
            offset = skip_quoted(query, offset);
            continue;
        }
        if query[offset] == b'#' {
            offset = skip_comment(query, offset);
            continue;
        }
        if &query[offset..offset + name.len()] == name
            && (offset == 0 || !is_identifier_byte(query[offset - 1]))
            && (offset + name.len() == query.len()
                || !is_identifier_byte(query[offset + name.len()]))
            && (offset == 0 || query[offset - 1] != b'.')
            && (offset == 0 || query[offset - 1] != b'$')
            && (offset < 2 || &query[offset - 2..offset] != b"::")
        {
            let mut previous = offset;
            while previous > 0 && query[previous - 1].is_ascii_whitespace() {
                previous -= 1;
            }
            let previous_end = previous;
            while previous > 0 && is_identifier_byte(query[previous - 1]) {
                previous -= 1;
            }
            if &query[previous..previous_end] == b"def" {
                offset += name.len();
                continue;
            }
            let mut after = offset + name.len();
            while after < query.len() && query[after].is_ascii_whitespace() {
                after += 1;
            }
            if arity == 0 {
                if after == query.len() || (query[after] != b'(' && query[after] != b':') {
                    return true;
                }
            } else if after < query.len()
                && query[after] == b'('
                && let Some(closing) = matching_parenthesis(query, after)
                && call_argument_count(query, after, closing) == arity
            {
                return true;
            }
        }
        offset += 1;
    }
    false
}

fn focused_function_body<'a>(source: &'a str, test_name: &str) -> Option<&'a str> {
    let marker = format!("fn {test_name}(");
    let start = source.find(&marker)?;
    let remainder = &source[start..];
    let end = [remainder.find("\n#[test]"), remainder.find("\nfn ")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(remainder.len());
    Some(&remainder[..end])
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
// This gate validates the complete source-to-catalog ledger in one pass so a
// missing relationship cannot hide behind separately passing subsets.
#[allow(clippy::too_many_lines)]
fn every_documented_signature_points_at_a_catalog_behavior_witness() {
    let ledger: serde_json::Value = tq_test_support::fixture_data::read(
        &root().join("tests/compatibility/reviews/jq-manual/completeness.toon"),
    )
    .unwrap();
    let rows = ledger["requirements"].as_array().unwrap();
    assert_eq!(rows.len(), 220);

    let catalog = load_catalog(&root().join("tests/compatibility/cases")).unwrap();
    let case_ids = catalog
        .cases
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut signatures = BTreeSet::new();
    for row in rows {
        let signature = row["signature"].as_str().unwrap();
        assert!(
            signatures.insert(signature),
            "duplicate signature: {signature}"
        );
        assert_eq!(row["status"], "semantic-witness");
        let evidence = row["evidence_ref"].as_str().unwrap();
        let witness = row["witness"].as_str().unwrap();
        let (name, arity) = signature.rsplit_once('/').unwrap();
        let arity: usize = arity.parse().unwrap();
        match row["evidence_kind"].as_str().unwrap() {
            "catalog-case" => {
                assert!(
                    case_ids.contains(evidence),
                    "{signature} points at missing catalog case {evidence}"
                );
                let case = catalog
                    .cases
                    .iter()
                    .find(|case| case.id == evidence)
                    .unwrap();
                assert_eq!(
                    case.status,
                    CaseStatus::Mvp,
                    "{signature} points at a non-MVP catalog case {evidence}"
                );
                assert!(
                    case.adapters.jq.supported && case.adapters.tq.supported,
                    "{signature} points at an unsupported catalog case {evidence}"
                );
                let query = &case.query;
                assert!(
                    query_calls_arity(query, name, arity),
                    "{signature} witness {evidence} does not execute the documented arity"
                );
                assert!(
                    query.contains(witness),
                    "{signature} witness token {witness} missing from {evidence}"
                );
            }
            "focused-public-test" => {
                let (path, test_name) = evidence
                    .split_once("::")
                    .unwrap_or_else(|| panic!("focused witness lacks source/test: {evidence}"));
                let source = std::fs::read_to_string(root().join(path)).unwrap_or_else(|error| {
                    panic!("cannot read focused witness {evidence}: {error}")
                });
                let source = focused_function_body(&source, test_name)
                    .unwrap_or_else(|| panic!("focused witness function is missing: {evidence}"));
                assert!(
                    query_calls_arity(witness, name, arity),
                    "{signature} focused witness query {witness:?} does not encode its arity"
                );
                assert!(
                    source.contains(witness),
                    "{signature} witness query {witness:?} missing from {evidence}"
                );
                /*
                 * The source slice above is deliberately limited to the named
                 * test. A same-named helper or another test cannot satisfy the
                 * evidence row by accident.
                 */
                assert!(
                    source.contains(&format!("fn {test_name}(")),
                    "focused witness function is missing: {evidence}"
                );
            }
            kind => panic!("{signature} uses unknown evidence kind {kind}"),
        }
    }

    let clauses = ledger["clauses"].as_array().unwrap();
    let clause_ids = clauses
        .iter()
        .map(|clause| clause["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let evidence = ledger["clause_evidence"].as_array().unwrap();
    assert_eq!(evidence.len(), 71);
    let allowed_evidence_kinds = ["catalog-case", "focused-public-test"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut evidenced_clauses = BTreeSet::new();
    for row in evidence {
        let clause = row["clause_id"].as_str().unwrap();
        assert!(clause_ids.contains(clause), "unknown clause: {clause}");
        let evidence_kind = row["evidence_kind"].as_str().unwrap();
        assert!(
            allowed_evidence_kinds.contains(evidence_kind),
            "{clause} uses unknown evidence kind {evidence_kind}"
        );
        evidenced_clauses.insert(clause);
        let reference = row["evidence_ref"].as_str().unwrap();
        assert!(!reference.contains(';'));
        if evidence_kind == "catalog-case" {
            assert!(
                case_ids.contains(reference),
                "{clause} points at missing catalog case {reference}"
            );
            let case = catalog
                .cases
                .iter()
                .find(|case| case.id == reference)
                .unwrap();
            assert_eq!(
                case.status,
                CaseStatus::Mvp,
                "{clause} points at a non-MVP catalog case {reference}"
            );
            assert!(
                case.adapters.jq.supported && case.adapters.tq.supported,
                "{clause} points at an unsupported catalog case {reference}"
            );
        } else {
            let (path, test_name) = reference
                .split_once("::")
                .unwrap_or_else(|| panic!("focused clause witness lacks source/test: {reference}"));
            let source = std::fs::read_to_string(root().join(path)).unwrap_or_else(|error| {
                panic!("cannot read focused clause witness {reference}: {error}")
            });
            assert!(
                source.contains(&format!("fn {test_name}(")),
                "focused clause witness function is missing: {reference}"
            );
        }
    }
    for clause in &clause_ids {
        assert!(
            evidenced_clauses.contains(clause),
            "clause {clause} has no evidence row"
        );
    }
    for clause in clauses {
        assert!(!clause["evidence_ref"].as_str().unwrap().contains(';'));
    }
}

#[test]
fn arity_scanner_rejects_quoted_names_prefixes_and_wrong_calls() {
    assert!(query_calls_arity(r#"match("x"; "g")"#, "match", 2));
    assert!(!query_calls_arity(r#"match("x"; "g")"#, "match", 1));
    assert!(query_calls_arity("IN(1; (1, 2))", "IN", 2));
    assert!(!query_calls_arity(r#""match("x")""#, "match", 1));
    assert!(!query_calls_arity("addendum", "add", 0));
    assert!(!query_calls_arity(".add", "add", 0));
    assert!(!query_calls_arity("# add(1)\n.", "add", 1));
    assert!(!query_calls_arity("m::add(1)", "add", 1));
    assert!(!query_calls_arity("def add: .", "add", 0));
    assert!(!query_calls_arity("{add: 1}", "add", 0));
    assert!(query_calls_arity("add(.[].a)", "add", 1));
}
