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

fn change_id(path: &str) -> Option<String> {
    let rest = path.strip_prefix("openspec/changes/")?;
    let id = if let Some(archive) = rest.strip_prefix("archive/") {
        let folder = archive.split_once('/')?.0;
        // Archive names are YYYY-MM-DD-<change-id>.
        if folder.len() < 12
            || folder.as_bytes()[4] != b'-'
            || folder.as_bytes()[7] != b'-'
            || folder.as_bytes()[10] != b'-'
            || folder.as_bytes()[..10]
                .iter()
                .enumerate()
                .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
        {
            return None;
        }
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

fn synchronized(delta: &str, main: &str, previous: &str) -> Check<()> {
    for section in delta.lines().filter_map(|line| line.strip_prefix("## ")) {
        if !matches!(
            section,
            "ADDED Requirements"
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
                if main_map.get(requirement.name.as_str()) != Some(&normalize(&requirement.body)) {
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
        let deltas: Vec<_> = paths
            .iter()
            .filter(|p| p.starts_with(&format!("{archive}/specs/")) && p.ends_with("/spec.md"))
            .collect();
        if deltas.is_empty() {
            let explanation = git(
                root,
                &["show", &format!("{head}:{archive}/no-spec-deltas.md")],
            )
            .unwrap_or_default();
            if explanation.trim().is_empty() {
                return Err(format!(
                    "{id}: no spec deltas; add a reviewed no-spec-deltas.md explanation to the archive"
                ));
            }
        }
        for path in deltas {
            let suffix = path.strip_prefix(&format!("{archive}/specs/")).unwrap();
            let delta = git(root, &["show", &format!("{head}:{path}")])?;
            let main = git(root, &["show", &format!("{head}:openspec/specs/{suffix}")])
                .unwrap_or_default();
            let previous = git(
                root,
                &[
                    "show",
                    &format!("{}:openspec/specs/{suffix}", merge_base.trim()),
                ],
            )
            .unwrap_or_default();
            synchronized(&delta, &main, &previous).map_err(|e| format!("{id}/{suffix}: {e}"))?;
            covered_specs.insert(format!("openspec/specs/{suffix}"));
        }
        eprintln!("{id}: archived and synchronized; review any no-spec-deltas explanation.");
        selected.push(archive.clone());
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
        fn archive(&self, id: &str) {
            let root = format!("openspec/changes/archive/2026-09-06-{id}");
            self.write(&format!("{root}/proposal.md"), "A behavior change");
            self.write(&format!("{root}/tasks.md"), "- [x] Implement");
            self.write(&format!("{root}/specs/test/spec.md"), DELTA);
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
        repo.archive("selected");
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
