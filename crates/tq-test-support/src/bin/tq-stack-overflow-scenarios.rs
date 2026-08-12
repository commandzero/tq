//! Generate checked-in Stack Overflow scenarios from Stack Exchange snapshots.

use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

use serde::Deserialize;
use serde_json::{Value, json};

const EXPECTED_COUNT: usize = 50;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tq-stack-overflow-scenarios: {error}");
            ExitCode::from(2)
        }
    }
}

#[derive(Debug, Deserialize)]
struct Snapshot<T> {
    items: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct Question {
    #[serde(rename = "question_id")]
    id: u64,
    title: String,
    score: i64,
    link: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(rename = "answer_id")]
    id: u64,
    score: i64,
    #[serde(default)]
    is_accepted: bool,
    body: String,
}

#[derive(Debug, Deserialize)]
struct BenchmarkDefinition {
    query: String,
    input: Value,
}

struct Options {
    questions: PathBuf,
    answers: PathBuf,
    benchmarks: PathBuf,
    output: PathBuf,
    patch: PathBuf,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let options = options()?;
    let questions = load_questions(&options.questions)?;
    let answers = load_answers(&options.answers)?;
    let benchmarks = load_benchmarks(&options.benchmarks)?;

    if questions.len() != EXPECTED_COUNT || benchmarks.len() != EXPECTED_COUNT {
        return Err(format!(
            "expected {EXPECTED_COUNT} questions and benchmark definitions, got {} and {}",
            questions.len(),
            benchmarks.len()
        )
        .into());
    }

    let mut patch_text = String::from("*** Begin Patch\n");
    for (rank, (question, benchmark)) in questions.iter().zip(benchmarks).enumerate() {
        let (answer, selection) = choose_answer(question, &answers)?;
        let filename = format!("{:02}-{}.json", rank + 1, slug(&question.title));
        let scenario_path = options.output.join(filename);
        let scenario = json!({
            "schema_version": 1,
            "id": format!("stack-overflow.{:02}", rank + 1),
            "rank": rank + 1,
            "source": {
                "tag": "jq",
                "sort": "votes",
                "retrieved_from": "https://stackoverflow.com/questions/tagged/jq?tab=votes&pagesize=50",
                "question_id": question.id,
                "title": question.title,
                "score": question.score,
                "url": question.link,
                "body_html": question.body,
            },
            "answer": {
                "answer_id": answer.id,
                "score": answer.score,
                "accepted": answer.is_accepted,
                "selection": selection,
                "url": format!("https://stackoverflow.com/a/{}", answer.id),
                "body_html": answer.body,
            },
            "benchmark": {
                "input_format": "json",
                "query": benchmark.query,
                "input": benchmark.input,
            },
        });
        let content = serde_json::to_string_pretty(&scenario)?;
        patch_text.push_str(&patch_for(&scenario_path, &content));
        patch_text.push('\n');
    }
    patch_text.push_str("*** End Patch\n");

    if let Some(parent) = options.patch.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&options.patch, patch_text)?;
    println!(
        "generated {EXPECTED_COUNT} Stack Overflow scenarios in {}",
        options.patch.display()
    );
    Ok(())
}

fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut questions = None;
    let mut answers = None;
    let mut benchmarks = PathBuf::from("tests/stack-overflow-benchmarks.json");
    let mut output = PathBuf::from("tests/stack-overflow");
    let mut patch = None;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--questions" => {
                questions = Some(PathBuf::from(
                    arguments.next().ok_or("--questions requires a path")?,
                ));
            }
            "--answers" => {
                answers = Some(PathBuf::from(
                    arguments.next().ok_or("--answers requires a directory")?,
                ));
            }
            "--benchmarks" => {
                benchmarks = PathBuf::from(arguments.next().ok_or("--benchmarks requires a path")?);
            }
            "--output" => {
                output = PathBuf::from(arguments.next().ok_or("--output requires a directory")?);
            }
            "--patch" => {
                patch = Some(PathBuf::from(
                    arguments.next().ok_or("--patch requires a path")?,
                ));
            }
            "-h" | "--help" => {
                println!(
                    "Usage: tq-stack-overflow-scenarios --questions PATH --answers DIR --patch PATH [--benchmarks PATH] [--output DIR]"
                );
                std::process::exit(0);
            }
            value => return Err(format!("unknown argument: {value}").into()),
        }
    }
    Ok(Options {
        questions: questions.ok_or("--questions is required")?,
        answers: answers.ok_or("--answers is required")?,
        benchmarks,
        output,
        patch: patch.ok_or("--patch is required")?,
    })
}

fn load_questions(path: &Path) -> Result<Vec<Question>, Box<dyn std::error::Error>> {
    let snapshot: Snapshot<Question> = read_json(path)?;
    Ok(snapshot.items)
}

fn load_answers(
    directory: &Path,
) -> Result<BTreeMap<String, Vec<Answer>>, Box<dyn std::error::Error>> {
    let mut answer_files = BTreeMap::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let question_id = path
            .file_stem()
            .ok_or_else(|| invalid_file_name(&path))?
            .to_string_lossy()
            .into_owned();
        let snapshot: Snapshot<Answer> = read_json(&path)?;
        answer_files.insert(question_id, snapshot.items);
    }
    Ok(answer_files)
}

fn load_benchmarks(path: &Path) -> Result<Vec<BenchmarkDefinition>, Box<dyn std::error::Error>> {
    read_json(path)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn choose_answer<'a>(
    question: &Question,
    answer_files: &'a BTreeMap<String, Vec<Answer>>,
) -> Result<(&'a Answer, &'static str), Box<dyn std::error::Error>> {
    let question_id = question.id.to_string();
    let answers = answer_files
        .get(&question_id)
        .ok_or_else(|| format!("no answer snapshot found for question {question_id}"))?;
    if let Some(answer) = answers.iter().find(|answer| answer.is_accepted) {
        return Ok((answer, "accepted"));
    }
    answers
        .iter()
        .max_by_key(|answer| answer.score)
        .map(|answer| (answer, "highest-voted"))
        .ok_or_else(|| format!("no answers found for question {question_id}").into())
}

fn slug(title: &str) -> String {
    let mut value = String::new();
    let mut separator = false;
    for character in title.to_lowercase().chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            if separator && !value.is_empty() {
                value.push('-');
            }
            value.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    value.chars().take(60).collect()
}

fn patch_for(path: &Path, content: &str) -> String {
    let mut output = format!("*** Add File: {}\n", path.display());
    for (index, line) in content.lines().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push('+');
        output.push_str(line);
    }
    output
}

fn invalid_file_name(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("path has no file name: {}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slug_matches_fixture_naming_rules() {
        assert_eq!(slug("How do I parse JSON?"), "how-do-i-parse-json");
        assert_eq!(slug("jq: test / output"), "jq-test-output");
        assert!(slug(&"a".repeat(80)).len() <= 60);
    }
}
