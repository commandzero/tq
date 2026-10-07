//! Git-scoped repository policy. Native validators own document and task syntax.
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    path::Path,
    process::Command,
};

type Check<T> = Result<T, String>;

fn git(root: &Path, args: &[&str]) -> Check<String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id != "archive"
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn archive_date(folder: &str) -> Option<&str> {
    let date = folder.get(..10)?;
    (folder.as_bytes().get(10) == Some(&b'-')
        && date.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        }))
    .then_some(date)
}

fn change_id(path: &str) -> Option<String> {
    let rest = path.strip_prefix("openspec/changes/")?;
    let id = if let Some(archive) = rest.strip_prefix("archive/") {
        let folder = archive.split_once('/')?.0;
        // Archive names are YYYY-MM-DD-<change-id>.
        archive_date(folder)?;
        folder.get(11..)?
    } else {
        rest.split_once('/')?.0
    };
    valid_id(id).then(|| id.to_owned())
}

#[derive(Debug)]
struct Requirement {
    mode: String,
    name: String,
    body: String,
}

fn requirements(text: &str) -> Check<Vec<Requirement>> {
    let mut mode = String::new();
    let mut result: Vec<Requirement> = Vec::new();
    let mut current = None;
    for line in text.lines() {
        if let Some(section) = line.strip_prefix("## ") {
            mode = section.to_owned();
            current = None;
        } else if let Some(name) = line.strip_prefix("### Requirement: ") {
            if result.iter().any(|r| r.name == name) {
                return Err(format!("Duplicate requirement: {name}"));
            }
            result.push(Requirement {
                mode: mode.clone(),
                name: name.to_owned(),
                body: String::new(),
            });
            current = Some(result.len() - 1);
        } else if let Some(i) = current {
            result[i].body.push_str(line);
            result[i].body.push('\n');
        }
    }
    Ok(result)
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct ArchivedDelta {
    archive: String,
    suffix: String,
    text: String,
    requirements: Vec<Requirement>,
}

impl ArchivedDelta {
    fn date(&self) -> &str {
        archive_date(self.archive.rsplit('/').next().unwrap()).unwrap()
    }
}

fn expected_bodies(
    delta: &ArchivedDelta,
    deltas: &[ArchivedDelta],
) -> Check<BTreeMap<String, String>> {
    let mut expected = BTreeMap::new();
    for requirement in delta.requirements.iter().filter(|r| {
        matches!(
            r.mode.as_str(),
            "ADDED Requirements" | "MODIFIED Requirements"
        )
    }) {
        let mut dated = BTreeMap::<&str, Vec<&Requirement>>::new();
        for candidate in deltas
            .iter()
            .filter(|d| d.suffix == delta.suffix && d.date() >= delta.date())
        {
            for successor in candidate.requirements.iter().filter(|r| {
                r.name == requirement.name
                    && matches!(
                        r.mode.as_str(),
                        "ADDED Requirements" | "MODIFIED Requirements"
                    )
            }) {
                dated.entry(candidate.date()).or_default().push(successor);
            }
        }
        let mut body = normalize(&requirement.body);
        for (date, successors) in dated {
            let next = normalize(&successors[0].body);
            if successors.iter().any(|r| normalize(&r.body) != next) {
                return Err(format!(
                    "{}: conflicting requirement bodies on archive date {date}",
                    requirement.name
                ));
            }
            if date > delta.date() {
                // Only an explicit, strictly later MODIFIED operation can replace a body.
                if next != body && successors.iter().any(|r| r.mode == "ADDED Requirements") {
                    return Err(format!(
                        "{}: later ADDED requirement cannot supersede an archived body",
                        requirement.name
                    ));
                }
                if successors.iter().any(|r| r.mode == "MODIFIED Requirements") {
                    body = next;
                }
            }
        }
        expected.insert(requirement.name.clone(), body);
    }
    Ok(expected)
}

#[cfg(test)]
fn synchronized(delta: &str, main: &str, previous: &str) -> Check<()> {
    synchronized_with_expected(delta, main, previous, &BTreeMap::new())
}

fn synchronized_with_expected(
    delta: &str,
    main: &str,
    previous: &str,
    expected: &BTreeMap<String, String>,
) -> Check<()> {
    for section in delta.lines().filter_map(|line| line.strip_prefix("## ")) {
        if !matches!(
            section,
            "Purpose"
                | "ADDED Requirements"
                | "MODIFIED Requirements"
                | "REMOVED Requirements"
                | "RENAMED Requirements"
        ) {
            return Err(format!("Unsupported delta section {section:?}"));
        }
    }
    let main_requirements = requirements(main)?;
    let main_map: BTreeMap<_, _> = main_requirements
        .iter()
        .map(|r| (r.name.as_str(), normalize(&r.body)))
        .collect();
    let delta_requirements = requirements(delta)?;
    let previous_requirements = requirements(previous)?;
    let previous_map: BTreeMap<_, _> = previous_requirements
        .iter()
        .map(|r| (r.name.as_str(), normalize(&r.body)))
        .collect();
    let modified: BTreeSet<_> = delta_requirements
        .iter()
        .filter(|r| r.mode == "MODIFIED Requirements")
        .map(|r| r.name.clone())
        .collect();
    let mut checked = 0;
    for requirement in delta_requirements {
        match requirement.mode.as_str() {
            "ADDED Requirements" | "MODIFIED Requirements" => {
                if !requirement.body.contains("#### Scenario:") {
                    return Err(format!("{} has no scenario", requirement.name));
                }
                let body = expected
                    .get(&requirement.name)
                    .cloned()
                    .unwrap_or_else(|| normalize(&requirement.body));
                if main_map.get(requirement.name.as_str()) != Some(&body) {
                    return Err(format!(
                        "{} is missing or differs from the archived requirement and scenarios",
                        requirement.name
                    ));
                }
            }
            "REMOVED Requirements" => {
                if main_map.contains_key(requirement.name.as_str()) {
                    return Err(format!(
                        "{} must be removed from the main spec",
                        requirement.name
                    ));
                }
            }
            other => {
                return Err(format!(
                    "Unsupported delta section {other:?}; use ADDED, MODIFIED, REMOVED, or RENAMED Requirements"
                ));
            }
        }
        checked += 1;
    }
    let mut renamed = false;
    let mut from = None;
    for line in delta.lines() {
        if let Some(section) = line.strip_prefix("## ") {
            renamed = section == "RENAMED Requirements";
        }
        if !renamed {
            continue;
        }
        if let Some(name) = line
            .strip_prefix("- FROM: `### Requirement: ")
            .and_then(|s| s.strip_suffix('`'))
        {
            if from.replace(name).is_some() {
                return Err("Rename is missing its TO entry".into());
            }
        } else if let Some(name) = line
            .strip_prefix("- TO: `### Requirement: ")
            .and_then(|s| s.strip_suffix('`'))
        {
            let old = from.take().ok_or("Rename is missing its FROM entry")?;
            if main_map.contains_key(old) || !main_map.contains_key(name) {
                return Err(format!("Rename {old} -> {name} is not synchronized"));
            }
            // A rename may already be synchronized at the merge base. In that
            // case the destination, rather than the source, establishes its body.
            if previous_map.contains_key(old) == previous_map.contains_key(name) {
                return Err(format!(
                    "Rename {old} -> {name} needs exactly one prior name"
                ));
            }
            if !modified.contains(name)
                && previous_map.get(old).or_else(|| previous_map.get(name)) != main_map.get(name)
            {
                return Err(format!(
                    "Rename {old} -> {name} changes or cannot establish the body; include a MODIFIED requirement under the new name"
                ));
            }
            checked += 1;
        } else if line.starts_with("- FROM:") || line.starts_with("- TO:") {
            return Err("Malformed rename entry".into());
        }
    }
    if from.is_some() || checked == 0 {
        return Err("Delta has no complete supported operations".into());
    }
    Ok(())
}

fn openspec(root: &Path, base: &str, head: &str) -> Check<Vec<String>> {
    // Resolve revisions first, so neither revision can be interpreted as a Git option.
    let base = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{base}^{{commit}}"),
        ],
    )?;
    let head = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{head}^{{commit}}"),
        ],
    )?;
    let head = head.trim();
    let merge_base = git(root, &["merge-base", base.trim(), head])?;
    // --no-renames emits both sides as deletion/addition and covers deleted-only changes.
    let changed = git(
        root,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            merge_base.trim(),
            head,
            "--",
            "openspec/changes",
            "openspec/specs",
        ],
    )?;
    let tree = git(
        root,
        &["ls-tree", "-r", "--name-only", "-z", head, "--", "openspec"],
    )?;
    let paths: Vec<_> = tree.split('\0').filter(|p| !p.is_empty()).collect();
    // Associations come from committed OpenSpec paths in the PR diff. This
    // keeps the gate tied to reviewable repository state rather than mutable
    // PR description text.
    let mut ids = BTreeSet::new();
    let mut changed_specs = BTreeSet::new();
    for path in changed.split('\0').filter(|p| !p.is_empty()) {
        if path.starts_with("openspec/specs/") {
            changed_specs.insert(path.to_owned());
            continue;
        }
        if let Some(id) = change_id(path) {
            ids.insert(id);
        } else {
            return Err(format!(
                "Cannot identify the OpenSpec change for {path}; use lowercase change IDs and YYYY-MM-DD-<id> archive directories"
            ));
        }
    }
    let mut selected = Vec::new();
    let mut covered_specs = BTreeSet::new();
    for id in &ids {
        if paths
            .iter()
            .any(|p| p.starts_with(&format!("openspec/changes/{id}/")))
        {
            return Err(format!(
                "{id}: active change remains; synchronize and archive it before merge"
            ));
        }
        let archives: BTreeSet<_> = paths
            .iter()
            .filter(|p| {
                p.starts_with("openspec/changes/archive/") && change_id(p).as_ref() == Some(id)
            })
            .map(|p| p.split('/').take(4).collect::<Vec<_>>().join("/"))
            .collect();
        if archives.len() != 1 {
            return Err(format!(
                "{id}: expected exactly one matching archive, found {}",
                archives.len()
            ));
        }
        let archive = archives.first().unwrap();
        for required in ["proposal.md", "tasks.md"] {
            if !paths.contains(&format!("{archive}/{required}").as_str()) {
                return Err(format!("{id}: archive is missing {required}"));
            }
        }
        // Preserve baseline artifacts and artifacts introduced by PR commits,
        // even when their active paths disappeared before the final diff.
        let mut old = git(
            root,
            &[
                "ls-tree",
                "-r",
                "--name-only",
                "-z",
                merge_base.trim(),
                "--",
                &format!("openspec/changes/{id}/"),
            ],
        )?;
        old.push_str(&git(
            root,
            &[
                "log",
                "--format=",
                "--name-only",
                "-z",
                "--no-renames",
                "--diff-filter=AM",
                &format!("{}..{head}", merge_base.trim()),
                "--",
                &format!("openspec/changes/{id}/"),
            ],
        )?);
        for path in old.split('\0').filter(|p| !p.is_empty()) {
            let suffix = path
                .strip_prefix(&format!("openspec/changes/{id}/"))
                .unwrap();
            if !paths.contains(&format!("{archive}/{suffix}").as_str()) {
                return Err(format!("{id}: archive lost artifact {suffix}"));
            }
        }
        selected.push(archive.clone());
    }
    // Collect successors only after committed-diff association and unique archive
    // selection. Historical archives outside this PR cannot waive synchronization.
    let mut deltas = Vec::new();
    for archive in &selected {
        let delta_paths: Vec<_> = paths
            .iter()
            .filter(|p| p.starts_with(&format!("{archive}/specs/")) && p.ends_with("/spec.md"))
            .collect();
        if delta_paths.is_empty() {
            let explanation = git(
                root,
                &["show", &format!("{head}:{archive}/no-spec-deltas.md")],
            )
            .unwrap_or_default();
            if explanation.trim().is_empty() {
                return Err(format!(
                    "{archive}: no spec deltas; add a reviewed no-spec-deltas.md explanation to the archive"
                ));
            }
        }
        for path in delta_paths {
            let suffix = path.strip_prefix(&format!("{archive}/specs/")).unwrap();
            let text = git(root, &["show", &format!("{head}:{path}")])?;
            deltas.push(ArchivedDelta {
                archive: archive.clone(),
                suffix: suffix.to_owned(),
                requirements: requirements(&text)
                    .map_err(|e| format!("{archive}/{suffix}: {e}"))?,
                text,
            });
        }
    }
    for delta in &deltas {
        let suffix = &delta.suffix;
        let main =
            git(root, &["show", &format!("{head}:openspec/specs/{suffix}")]).unwrap_or_default();
        let previous = git(
            root,
            &[
                "show",
                &format!("{}:openspec/specs/{suffix}", merge_base.trim()),
            ],
        )
        .unwrap_or_default();
        let expected = expected_bodies(delta, &deltas)
            .map_err(|e| format!("{}/{suffix}: {e}", delta.archive))?;
        synchronized_with_expected(&delta.text, &main, &previous, &expected)
            .map_err(|e| format!("{}/{suffix}: {e}", delta.archive))?;
        covered_specs.insert(format!("openspec/specs/{suffix}"));
    }
    for archive in &selected {
        eprintln!("{archive}: archived and synchronized; review any no-spec-deltas explanation.");
    }
    if let Some(path) = changed_specs.difference(&covered_specs).next() {
        return Err(format!(
            "{path}: main-spec edits require a corresponding delta in an associated archive"
        ));
    }
    if ids.is_empty() {
        eprintln!("OpenSpec completion: not applicable.");
    }
    Ok(selected)
}

fn docs_only(path: &str) -> bool {
    path.starts_with("docs/") || (!path.contains('/') && path.ends_with(".md"))
}

fn scope(root: &Path, base: &str, head: &str) -> Check<()> {
    let base = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{base}^{{commit}}"),
        ],
    )?;
    let head = git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{head}^{{commit}}"),
        ],
    )?;
    let merge_base = git(root, &["merge-base", base.trim(), head.trim()])?;
    let changed = git(
        root,
        &[
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            merge_base.trim(),
            head.trim(),
            "--",
        ],
    )?;
    println!(
        "{}",
        if changed.split('\0').filter(|s| !s.is_empty()).all(docs_only) {
            "docs"
        } else {
            "code"
        }
    );
    Ok(())
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let root = Path::new(".");
    let result = match args.get(1).map(String::as_str) {
        Some("scope") if args.len() == 4 => scope(root, &args[2], &args[3]),
        Some("openspec") if args.len() == 4 => openspec(root, &args[2], &args[3]).map(|archives| {
            for archive in archives {
                println!("{archive}");
            }
        }),
        _ => Err(concat!("Usage: repo-check.sh scope BASE HEAD | openspec BASE HEAD").into()),
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::{fs, path::PathBuf};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Repo(PathBuf);
    impl Repo {
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "tq-repo-check-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            git(&path, &["init", "-q"]).unwrap();
            git(&path, &["config", "user.name", "Repository test"]).unwrap();
            git(&path, &["config", "user.email", "test@example.invalid"]).unwrap();
            Self(path)
        }
        fn write(&self, path: &str, body: &str) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, body).unwrap();
        }
        fn commit(&self) -> String {
            git(&self.0, &["add", "."]).unwrap();
            git(
                &self.0,
                &[
                    "-c",
                    "commit.gpgsign=false",
                    "commit",
                    "--allow-empty",
                    "-qm",
                    "test",
                ],
            )
            .unwrap();
            git(&self.0, &["rev-parse", "HEAD"]).unwrap().trim().into()
        }
        fn archive_at(&self, date: &str, id: &str, delta: &str) {
            let root = format!("openspec/changes/archive/{date}-{id}");
            self.write(&format!("{root}/proposal.md"), "A behavior change");
            self.write(&format!("{root}/tasks.md"), "- [x] Implement");
            self.write(&format!("{root}/specs/test/spec.md"), delta);
        }
        fn archive(&self, id: &str) {
            self.archive_at("2026-09-06", id, DELTA);
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    const MAIN: &str = "## Requirements\n### Requirement: Behavior\nThe system SHALL work.\n#### Scenario: Input\n- **WHEN** input arrives\n- **THEN** output is correct\n";
    const DELTA: &str = "## ADDED Requirements\n### Requirement: Behavior\nThe system SHALL work.\n#### Scenario: Input\n- **WHEN** input arrives\n- **THEN** output is correct\n";
    #[test]
    fn no_open_spec_changes_need_no_association() {
        let repo = Repo::new();
        let base = repo.commit();
        repo.write("README.md", "docs");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
    }
    #[test]
    fn archive_dates_require_numeric_components() {
        for date in ["abcd-ef-ij", "2026-XX-06", "2026-09-XX", "é026-09-06"] {
            assert!(
                change_id(&format!("openspec/changes/archive/{date}-change/tasks.md")).is_none()
            );
        }
        assert_eq!(
            change_id("openspec/changes/archive/2026-09-15-change/tasks.md"),
            Some("change".into())
        );
    }
    #[test]
    fn unknown_delta_sections_fail_even_without_requirements() {
        for extra in [
            "## OTHER Requirements\nIgnored text\n",
            "## OTHER Requirements\n",
        ] {
            assert!(synchronized(&format!("{DELTA}{extra}"), MAIN, "").is_err());
        }
    }
    #[test]
    fn purpose_metadata_is_allowed_without_replacing_delta_operations() {
        let purpose = "## Purpose\nDefine a new capability with its observable behavior.\n\n";
        assert!(
            synchronized(
                &format!("{purpose}{DELTA}"),
                &format!("{purpose}{MAIN}"),
                ""
            )
            .is_ok()
        );
        assert!(synchronized(purpose, &format!("{purpose}{MAIN}"), "").is_err());
        assert!(
            synchronized(
                &format!("{purpose}{DELTA}## OTHER Requirements\n"),
                MAIN,
                ""
            )
            .is_err()
        );
        assert!(synchronized(&format!("{purpose}{DELTA}"), "", "").is_err());
    }
    #[test]
    fn detects_missing_or_changed_scenarios() {
        assert!(synchronized(DELTA, MAIN, "").is_ok());
        assert!(synchronized(DELTA, &MAIN.replace("correct", "wrong"), "").is_err());
        assert!(synchronized(DELTA, "", "").is_err());
    }
    #[test]
    fn modified_removed_and_renamed_contracts() {
        assert!(synchronized(&DELTA.replace("ADDED", "MODIFIED"), MAIN, "").is_ok());
        let removed = "## REMOVED Requirements\n### Requirement: Behavior\nReason: obsolete\n";
        assert!(synchronized(removed, "", MAIN).is_ok());
        assert!(synchronized(removed, MAIN, MAIN).is_err());
        let renamed = "## RENAMED Requirements\n- FROM: `### Requirement: Old`\n- TO: `### Requirement: Behavior`\n";
        assert!(synchronized(renamed, MAIN, &MAIN.replace("Behavior", "Old")).is_ok());
        assert!(synchronized(renamed, "", MAIN).is_err());
        assert!(synchronized(renamed, MAIN, MAIN).is_ok());
        let changed = MAIN.replace("correct", "wrong");
        let added = format!("{renamed}{}", DELTA.replace("correct", "wrong"));
        assert!(synchronized(&added, &changed, &MAIN.replace("Behavior", "Old")).is_err());
        let modified = added.replace("ADDED Requirements", "MODIFIED Requirements");
        assert!(synchronized(&modified, &changed, &MAIN.replace("Behavior", "Old")).is_ok());
        assert!(synchronized(&modified, &changed, "").is_err());
        assert!(
            synchronized(
                &modified,
                &changed,
                &format!("{MAIN}{}", MAIN.replace("Behavior", "Old"))
            )
            .is_err()
        );
        assert!(
            synchronized(
                renamed,
                &MAIN.replace("correct", "wrong"),
                &MAIN.replace("Behavior", "Old")
            )
            .is_err()
        );
    }
    #[test]
    fn unrelated_active_changes_do_not_block() {
        let repo = Repo::new();
        repo.write("openspec/changes/unrelated/tasks.md", "- [ ] Work");
        let base = repo.commit();
        repo.write("README.md", "docs");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
    }
    #[test]
    fn unrecognized_change_paths_fail_closed() {
        for path in [
            "openspec/changes/loose.md",
            "openspec/changes/loose",
            "openspec/changes/archive/loose.md",
            "openspec/changes/archive/2026-09-15-loose",
        ] {
            let repo = Repo::new();
            let base = repo.commit();
            repo.write(path, "Unassociated change");
            let head = repo.commit();
            assert!(openspec(&repo.0, &base, &head).is_err());
        }
    }
    #[test]
    fn main_spec_edits_require_archived_deltas() {
        let repo = Repo::new();
        let base = repo.commit();
        repo.write("openspec/specs/test/spec.md", MAIN);
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
        repo.archive("new-spec");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
        repo.write("openspec/specs/unrelated/spec.md", MAIN);
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
    }
    #[test]
    fn unapplied_new_spec_delta_is_not_synchronization() {
        let repo = Repo::new();
        let base = repo.commit();
        repo.archive("new-spec");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
    }
    #[test]
    fn edited_deleted_and_renamed_active_changes_fail() {
        let repo = Repo::new();
        repo.write("openspec/changes/change/tasks.md", "- [ ] Work");
        let base = repo.commit();
        repo.write("openspec/changes/change/tasks.md", "- [x] Work");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
        fs::rename(
            repo.0.join("openspec/changes/change"),
            repo.0.join("openspec/changes/renamed"),
        )
        .unwrap();
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
        fs::remove_dir_all(repo.0.join("openspec/changes")).unwrap();
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
    }
    #[test]
    fn new_archives_and_previously_synced_specs_pass() {
        let repo = Repo::new();
        repo.write("openspec/specs/test/spec.md", MAIN);
        let base = repo.commit();
        repo.archive("first");
        repo.archive("second");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
        repo.write(
            "openspec/specs/test/spec.md",
            &MAIN.replace("correct", "wrong"),
        );
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
    }

    #[test]
    fn supersession_uses_dates_not_ids_and_checks_unaffected_requirements() {
        for mode in ["ADDED", "MODIFIED"] {
            let repo = Repo::new();
            let base = repo.commit();
            let unaffected = DELTA.replace("Behavior", "Unaffected");
            repo.archive_at(
                "2026-10-04",
                "output-colors",
                &format!("{}{unaffected}", DELTA.replace("ADDED", mode)),
            );
            let latest = DELTA
                .replace("ADDED", "MODIFIED")
                .replace("correct", "latest");
            repo.archive_at("2026-10-06", "achieve-jq-manual-parity", &latest);
            let main = format!(
                "{}{}",
                MAIN.replace("correct", "latest"),
                MAIN.replace("Behavior", "Unaffected")
            );
            repo.write("openspec/specs/test/spec.md", &main);
            let head = repo.commit();
            let selected = openspec(&repo.0, &base, &head).unwrap();
            assert_eq!(selected.len(), 2);
            assert!(selected.iter().any(|p| p.ends_with("output-colors")));
            repo.write(
                "openspec/specs/test/spec.md",
                &main.replace("Requirement: Unaffected", "Requirement: Missing"),
            );
            let head = repo.commit();
            assert!(
                openspec(&repo.0, &base, &head)
                    .unwrap_err()
                    .contains("Unaffected")
            );
        }
    }

    #[test]
    fn supersession_allows_three_later_modifications() {
        let repo = Repo::new();
        let base = repo.commit();
        repo.archive_at("2026-10-01", "z-original", DELTA);
        for (date, id, body) in [
            ("2026-10-02", "y-first", "first"),
            ("2026-10-03", "x-second", "second"),
            ("2026-10-04", "a-final", "final"),
        ] {
            repo.archive_at(
                date,
                id,
                &DELTA.replace("ADDED", "MODIFIED").replace("correct", body),
            );
        }
        repo.write(
            "openspec/specs/test/spec.md",
            &MAIN.replace("correct", "final"),
        );
        let head = repo.commit();
        assert_eq!(openspec(&repo.0, &base, &head).unwrap().len(), 4);
    }

    #[test]
    fn supersession_rejects_added_stale_same_date_and_wrong_identity() {
        for case in [
            "added",
            "stale",
            "same-date",
            "capability",
            "name",
            "scenario",
        ] {
            let repo = Repo::new();
            let base = repo.commit();
            repo.archive_at("2026-10-04", "z-original", DELTA);
            let mut later = DELTA
                .replace("ADDED", "MODIFIED")
                .replace("correct", "latest");
            if case == "added" {
                later = later.replace("MODIFIED", "ADDED");
            }
            if case == "name" {
                later = later.replace("Behavior", "Other");
            }
            if case == "scenario" {
                later = later.replace("Scenario: Input", "Scenario: Different");
            }
            let date = if case == "same-date" {
                "2026-10-04"
            } else {
                "2026-10-06"
            };
            repo.archive_at(date, "a-later", &later);
            if case == "capability" {
                fs::rename(
                    repo.0.join(format!(
                        "openspec/changes/archive/{date}-a-later/specs/test"
                    )),
                    repo.0.join(format!(
                        "openspec/changes/archive/{date}-a-later/specs/other"
                    )),
                )
                .unwrap();
                repo.write(
                    "openspec/specs/other/spec.md",
                    &MAIN.replace("correct", "latest"),
                );
            }
            let mut main = if case == "stale" {
                MAIN.to_owned()
            } else {
                MAIN.replace("correct", "latest")
            };
            if case == "name" {
                main.push_str(
                    &MAIN
                        .replace("Behavior", "Other")
                        .replace("correct", "latest"),
                );
            }
            repo.write("openspec/specs/test/spec.md", &main);
            let head = repo.commit();
            assert!(openspec(&repo.0, &base, &head).is_err(), "{case}");
        }
    }

    #[test]
    fn final_modification_cannot_mask_conflicting_intermediate_operations() {
        for case in ["same-date", "added"] {
            let repo = Repo::new();
            let base = repo.commit();
            repo.archive_at("2026-10-04", "z-original", DELTA);
            let date = if case == "same-date" {
                "2026-10-04"
            } else {
                "2026-10-05"
            };
            let mode = if case == "added" { "ADDED" } else { "MODIFIED" };
            repo.archive_at(
                date,
                "y-intermediate",
                &DELTA
                    .replace("ADDED", mode)
                    .replace("correct", "intermediate"),
            );
            repo.archive_at(
                "2026-10-06",
                "a-final",
                &DELTA
                    .replace("ADDED", "MODIFIED")
                    .replace("correct", "final"),
            );
            repo.write(
                "openspec/specs/test/spec.md",
                &MAIN.replace("correct", "final"),
            );
            let head = repo.commit();
            let error = openspec(&repo.0, &base, &head).unwrap_err();
            let diagnostic = if case == "same-date" {
                "conflicting requirement bodies"
            } else {
                "later ADDED requirement cannot supersede"
            };
            assert!(error.contains(diagnostic), "{case}: {error}");
        }
    }

    #[test]
    fn same_date_identical_requirements_pass_with_normalization() {
        let repo = Repo::new();
        let base = repo.commit();
        repo.archive_at("2026-10-04", "z-original", DELTA);
        repo.archive_at(
            "2026-10-04",
            "a-identical",
            &DELTA
                .replace("ADDED", "MODIFIED")
                .replace("SHALL work", "SHALL   work"),
        );
        repo.write("openspec/specs/test/spec.md", MAIN);
        let head = repo.commit();
        assert_eq!(openspec(&repo.0, &base, &head).unwrap().len(), 2);
    }

    #[test]
    fn unassociated_historical_archive_cannot_supersede() {
        let repo = Repo::new();
        let latest = DELTA
            .replace("ADDED", "MODIFIED")
            .replace("correct", "latest");
        repo.archive_at("2026-10-06", "a-history", &latest);
        repo.write(
            "openspec/specs/test/spec.md",
            &MAIN.replace("correct", "latest"),
        );
        let base = repo.commit();
        repo.archive_at("2026-10-04", "z-original", DELTA);
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
    }

    #[test]
    fn supersession_still_validates_older_and_later_delta_syntax() {
        for bad in ["older", "later"] {
            for (malformed, diagnostic) in [
                (
                    DELTA.replace("#### Scenario:", "#### Example:"),
                    "has no scenario",
                ),
                (
                    format!("{DELTA}## UNKNOWN Requirements\n"),
                    "Unsupported delta section",
                ),
                (format!("{DELTA}{DELTA}"), "Duplicate requirement"),
            ] {
                let repo = Repo::new();
                let base = repo.commit();
                let latest = DELTA
                    .replace("ADDED", "MODIFIED")
                    .replace("correct", "latest");
                let malformed = if bad == "later" {
                    malformed
                        .replace("ADDED", "MODIFIED")
                        .replace("correct", "latest")
                } else {
                    malformed
                };
                repo.archive_at(
                    "2026-10-04",
                    "z-original",
                    if bad == "older" { &malformed } else { DELTA },
                );
                repo.archive_at(
                    "2026-10-06",
                    "a-later",
                    if bad == "later" { &malformed } else { &latest },
                );
                let mut main = MAIN.replace("correct", "latest");
                if bad == "later" && diagnostic == "has no scenario" {
                    main = main.replace("#### Scenario:", "#### Example:");
                }
                repo.write("openspec/specs/test/spec.md", &main);
                let head = repo.commit();
                let error = openspec(&repo.0, &base, &head).unwrap_err();
                assert!(error.contains(diagnostic), "{bad}: {error}");
            }
        }
    }

    #[test]
    fn supersession_preserves_artifacts_in_both_archives() {
        for id in ["z-original", "a-later"] {
            let repo = Repo::new();
            repo.write(
                &format!("openspec/changes/{id}/design.md"),
                "Preserve design",
            );
            let base = repo.commit();
            fs::remove_dir_all(repo.0.join(format!("openspec/changes/{id}"))).unwrap();
            repo.archive_at("2026-10-04", "z-original", DELTA);
            repo.archive_at(
                "2026-10-06",
                "a-later",
                &DELTA
                    .replace("ADDED", "MODIFIED")
                    .replace("correct", "latest"),
            );
            repo.write(
                "openspec/specs/test/spec.md",
                &MAIN.replace("correct", "latest"),
            );
            let head = repo.commit();
            assert!(
                openspec(&repo.0, &base, &head)
                    .unwrap_err()
                    .contains("lost artifact design.md")
            );
            let date = if id == "z-original" {
                "2026-10-04"
            } else {
                "2026-10-06"
            };
            repo.write(
                &format!("openspec/changes/archive/{date}-{id}/design.md"),
                "Preserve design",
            );
            let head = repo.commit();
            assert_eq!(openspec(&repo.0, &base, &head).unwrap().len(), 2);
        }
    }

    #[test]
    fn native_task_gate_rejects_incomplete_selected_archives_only() {
        let repo = Repo::new();
        for script in [
            "openspec-check.sh",
            "repo-check.sh",
            "repo-check.rs",
            "tools-versions.sh",
        ] {
            let body = fs::read_to_string(Path::new("scripts").join(script)).unwrap();
            repo.write(&format!("scripts/{script}"), &body);
        }
        use std::os::unix::fs::PermissionsExt;
        for script in ["openspec-check.sh", "repo-check.sh"] {
            fs::set_permissions(
                repo.0.join("scripts").join(script),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        repo.archive("unrelated");
        repo.write(
            "openspec/changes/archive/2026-09-06-unrelated/tasks.md",
            "- [ ] Unrelated unfinished history\n",
        );
        repo.write("openspec/specs/test/spec.md", &format!("# Test\n\n## Purpose\nVerify repository policy without changing native task validation semantics.\n\n{MAIN}"));
        let base = repo.commit();
        repo.archive_at("2026-09-04", "z-original", DELTA);
        repo.archive_at(
            "2026-09-06",
            "selected",
            &DELTA
                .replace("ADDED", "MODIFIED")
                .replace("correct", "latest"),
        );
        repo.write("openspec/specs/test/spec.md", &format!("# Test\n\n## Purpose\nVerify repository policy without changing native task validation semantics.\n\n{}", MAIN.replace("correct", "latest")));
        repo.write(
            "openspec/changes/archive/2026-09-06-selected/tasks.md",
            "- [ ] Finish selected change\n",
        );
        let head = repo.commit();
        let run = |head: &str| {
            Command::new("bash")
                .args(["scripts/openspec-check.sh", &base, head])
                .current_dir(&repo.0)
                .env("OPENSPEC_TELEMETRY", "0")
                .output()
                .unwrap()
        };
        let incomplete = run(&head);
        assert!(
            !incomplete.status.success(),
            "native validation must reject incomplete tasks"
        );
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&incomplete.stdout),
            String::from_utf8_lossy(&incomplete.stderr)
        );
        assert!(diagnostic.contains("selected"), "{diagnostic}");
        repo.write(
            "openspec/changes/archive/2026-09-06-selected/tasks.md",
            "- [x] Finish selected change\n",
        );
        let head = repo.commit();
        let complete = run(&head);
        assert!(
            complete.status.success(),
            "{}{}",
            String::from_utf8_lossy(&complete.stdout),
            String::from_utf8_lossy(&complete.stderr)
        );
        repo.write(
            "openspec/changes/archive/2026-09-04-z-original/tasks.md",
            "- [ ] Finish older change\n",
        );
        let head = repo.commit();
        let older_incomplete = run(&head);
        assert!(!older_incomplete.status.success());
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&older_incomplete.stdout),
            String::from_utf8_lossy(&older_incomplete.stderr)
        );
        assert!(diagnostic.contains("z-original"), "{diagnostic}");
        repo.write(
            "openspec/changes/archive/2026-09-04-z-original/tasks.md",
            "- [x] Finish older change\n",
        );
        let head = repo.commit();
        assert!(run(&head).status.success());
        repo.write("openspec/specs/untracked/spec.md", MAIN);
        let untracked = run(&head);
        assert!(!untracked.status.success());
        assert!(String::from_utf8_lossy(&untracked.stderr).contains("Untracked OpenSpec"));
        fs::remove_dir_all(repo.0.join("openspec/specs/untracked")).unwrap();
        repo.write(
            "openspec/changes/archive/2026-09-06-selected/tasks.md",
            "- [ ] Uncommitted regression\n",
        );
        assert!(
            !run(&head).status.success(),
            "dirty specs cannot stand in for committed evidence"
        );
    }
    #[test]
    fn no_delta_requires_archived_explanation() {
        let repo = Repo::new();
        let base = repo.commit();
        repo.archive("change");
        fs::remove_dir_all(
            repo.0
                .join("openspec/changes/archive/2026-09-06-change/specs"),
        )
        .unwrap();
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
        repo.write(
            "openspec/changes/archive/2026-09-06-change/no-spec-deltas.md",
            "Documentation-only change; no product requirements change. Review with the PR.",
        );
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
    }
    #[test]
    fn archive_preserves_active_artifacts_and_checks_committed_state() {
        let repo = Repo::new();
        repo.write("openspec/changes/change/design.md", "Important design");
        repo.write("openspec/specs/test/spec.md", MAIN);
        let base = repo.commit();
        fs::remove_dir_all(repo.0.join("openspec/changes/change")).unwrap();
        repo.archive("change");
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
        repo.write(
            "openspec/changes/archive/2026-09-06-change/design.md",
            "Important design",
        );
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
        repo.write("openspec/specs/test/spec.md", "uncommitted change");
        assert!(openspec(&repo.0, &base, &head).is_ok());
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_err());
    }

    #[test]
    fn archive_preserves_artifacts_introduced_during_the_pr() {
        let repo = Repo::new();
        repo.write("openspec/specs/test/spec.md", MAIN);
        let base = repo.commit();
        repo.write("openspec/changes/change/design.md", "New design");
        repo.commit();
        fs::remove_dir_all(repo.0.join("openspec/changes/change")).unwrap();
        repo.archive("change");
        let head = repo.commit();
        let error = openspec(&repo.0, &base, &head).unwrap_err();
        assert!(error.contains("lost artifact design.md"), "{error}");
        repo.write(
            "openspec/changes/archive/2026-09-06-change/design.md",
            "New design",
        );
        let head = repo.commit();
        assert!(openspec(&repo.0, &base, &head).is_ok());
    }

    #[test]
    fn workflow_and_gate_edits_require_full_checks() {
        assert!(docs_only("docs/compatibility.md"));
        assert!(docs_only("README.md"));
        assert!(!docs_only(".github/workflows/preflight.yml"));
        assert!(!docs_only("scripts/repo-check.rs"));
        assert!(!docs_only("openspec/specs/test/spec.md"));
    }
}
