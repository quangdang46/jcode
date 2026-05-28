//! `jbench` CLI entry point.
//!
//! Dispatches to the [`jcode_jbench`] library for real work.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use jcode_jbench::{
    agent_runner::AgentRunConfig,
    judge::{JudgeConfig, judge_with_three_models},
    lessons::{LessonsConfig, append_lessons_to_file, extract_lessons},
    types::{AgentEvalResults, EvalDataV2, EvalRun},
};

/// Top-level `jbench` CLI.
#[derive(Debug, Parser)]
#[command(
    name = "jbench",
    about = "JBench — jcode's git-commit-reconstruction eval framework",
    version
)]
struct Cli {
    /// Subcommand to dispatch to.
    #[command(subcommand)]
    command: Command,
}

/// JBench subcommands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Select high-quality commits from a target repo to use as eval
    /// tasks.
    PickCommits {
        /// URL of the repository to pick commits from.
        repo_url: String,
        /// Minimum commit message length.
        #[arg(long, default_value = "10")]
        min_msg_len: usize,
        /// Maximum number of commits to pick.
        #[arg(long, default_value = "50")]
        max_picks: usize,
        /// Output file (default: stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Generate an `eval-{repo}.json` file (`EvalDataV2`) from a list
    /// of picked commits.
    GenEvals {
        /// Input commit list (from pick-commits).
        input: PathBuf,
        /// Output eval JSON file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Run one or more agents against an eval data file and emit
    /// per-commit `EvalRun`s.
    Run {
        /// Path to eval data JSON file.
        eval_file: PathBuf,
        /// Agent ID to run (must be registered in jcode registry).
        #[arg(short, long)]
        agent_id: String,
        /// Output directory for EvalRun JSON files.
        #[arg(short, long)]
        output_dir: PathBuf,
        /// Path to jcode binary (auto-detected if not set).
        #[arg(long)]
        jcode_binary: Option<PathBuf>,
        /// Maximum turns per run.
        #[arg(long, default_value = "100")]
        max_turns: u32,
        /// Timeout per run in seconds.
        #[arg(long, default_value = "3600")]
        timeout_secs: u64,
    },
    /// Re-judge an existing run with the three-judge median pipeline.
    Judge {
        /// Directory containing EvalRun JSON files.
        runs_dir: PathBuf,
        /// API base URL.
        #[arg(long, env = "JBENCH_API_BASE")]
        api_base: Option<String>,
        /// API key.
        #[arg(long, env = "JBENCH_API_KEY")]
        api_key: Option<String>,
    },
    /// Aggregate and analyze results across all tasks for an agent.
    MetaAnalyze {
        /// Directory containing EvalRun JSON files.
        runs_dir: PathBuf,
        /// Output file for aggregated results.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::PickCommits {
            repo_url,
            min_msg_len,
            max_picks,
            output,
        } => {
            pick_commits_impl(&repo_url, min_msg_len, max_picks, output).await?;
        }
        Command::GenEvals { input, output } => {
            gen_evals_impl(&input, &output).await?;
        }
        Command::Run {
            eval_file,
            agent_id,
            output_dir,
            jcode_binary,
            max_turns,
            timeout_secs,
        } => {
            run_impl(
                &eval_file,
                &agent_id,
                &output_dir,
                jcode_binary.as_ref(),
                max_turns,
                timeout_secs,
            )
            .await?;
        }
        Command::Judge {
            runs_dir,
            api_base,
            api_key,
        } => {
            judge_impl(&runs_dir, api_base.as_deref(), api_key.as_deref()).await?;
        }
        Command::MetaAnalyze { runs_dir, output } => {
            meta_analyze_impl(&runs_dir, output.as_ref()).await?;
        }
    }
    Ok(())
}

async fn pick_commits_impl(
    _repo_url: &str,
    _min_msg_len: usize,
    _max_picks: usize,
    _output: Option<PathBuf>,
) -> Result<()> {
    todo_step("Phase 5.2: commit selection via git log heuristics + message quality filter")
}

async fn gen_evals_impl(_input: &PathBuf, _output: &PathBuf) -> Result<()> {
    todo_step("Phase 5.2: read commit list, fetch each SHA, render EvalDataV2 JSON")
}

async fn run_impl(
    eval_file: &PathBuf,
    agent_id: &str,
    output_dir: &PathBuf,
    jcode_binary: Option<&PathBuf>,
    max_turns: u32,
    timeout_secs: u64,
) -> Result<()> {
    use std::fs;
    use std::time::Duration;
    use tokio::time::timeout as tk_timeout;

    // Load eval data
    let eval_data: EvalDataV2 = {
        let text = fs::read_to_string(eval_file)?;
        serde_json::from_str(&text).context("failed to parse eval JSON")?
    };

    if !output_dir.exists() {
        fs::create_dir_all(output_dir)?;
    }

    for commit in &eval_data.eval_commits {
        let config = AgentRunConfig {
            agent_id: agent_id.to_owned(),
            prompt: commit.prompt.clone(),
            repo_path: output_dir.join(&commit.id), // per-commit working dir
            max_turns,
            timeout_secs,
            env: eval_data.env.clone(),
            jcode_binary: jcode_binary.cloned(),
            ..Default::default()
        };

        let result = tk_timeout(
            Duration::from_secs(timeout_secs),
            jcode_jbench::agent_runner::run_agent_in_repo(config),
        )
        .await
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            Ok(jcode_jbench::types::EvalRun {
                commit_sha: commit.sha.clone(),
                prompt: commit.prompt.clone(),
                diff: String::new(),
                judging: Default::default(),
                cost_usd: 0.0,
                duration_ms: 0,
                error: Some("Timed out waiting for run_agent_in_repo".to_owned()),
            })
        })?;

        let run_file = output_dir.join(format!("{}.run.json", commit.id));
        let json = serde_json::to_string_pretty(&result).context("failed to serialize EvalRun")?;
        fs::write(&run_file, json)?;
        println!("Wrote {}", run_file.display());
    }

    Ok(())
}

async fn judge_impl(
    _runs_dir: &PathBuf,
    _api_base: Option<&str>,
    _api_key: Option<&str>,
) -> Result<()> {
    todo_step(
        "Phase 5.4: load EvalRun JSONs, call judge_with_three_models, overwrite judging fields",
    )
}

async fn meta_analyze_impl(runs_dir: &PathBuf, output: Option<&PathBuf>) -> Result<()> {
    use jcode_jbench::types::AgentEvalResults;
    use std::fs;

    let mut all_runs = Vec::new();

    for entry in fs::read_dir(runs_dir)? {
        let entry = entry?;
        let path = entry.path();
        // `Path::extension` returns only the trailing component (`json`),
        // so matching against `"run.json"` never fires. Match on the full
        // file name suffix instead.
        let is_run_file = path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.ends_with(".run.json"));
        if is_run_file {
            let text = fs::read_to_string(&path)?;
            if let Ok(run) = serde_json::from_str::<EvalRun>(&text) {
                all_runs.push(run);
            }
        }
    }

    if all_runs.is_empty() {
        anyhow::bail!("No .run.json files found in {}", runs_dir.display());
    }

    let avg_score = all_runs
        .iter()
        .map(|r| r.judging.overall_score)
        .sum::<f64>()
        / all_runs.len() as f64;
    let avg_cost = all_runs.iter().map(|r| r.cost_usd).sum::<f64>() / all_runs.len() as f64;
    let avg_duration = all_runs.iter().map(|r| r.duration_ms).sum::<u64>() / all_runs.len() as u64;

    let summary = AgentEvalResults {
        agent_id: "unknown".to_owned(),
        runs: all_runs,
        average_score: (avg_score * 10.0).round() / 10.0,
        average_cost: (avg_cost * 100.0).round() / 100.0,
        average_duration_ms: avg_duration,
    };

    let json = serde_json::to_string_pretty(&summary).context("failed to serialize summary")?;

    if let Some(out) = output {
        fs::write(out, &json)?;
        println!("Wrote {}", out.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

fn todo_step(phase: &str) -> Result<()> {
    eprintln!("{phase}");
    std::process::exit(0);
}
