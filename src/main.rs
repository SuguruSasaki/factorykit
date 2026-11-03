mod agent;
mod domain;
mod error;
mod github;
mod process;
mod state;
mod worker;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use agent::Claude;
use domain::{Config, Repository};
use error::{Error, Result};
use github::{GitHub, Queue};
use process::Runner;
use worker::Worker;

const HELP: &str = "factorykit — local approval-based development workflow (macOS/Unix)

Usage:
  factorykit setup --repository OWNER/REPO [--apply]
  factorykit worker --project PATH [--dry-run | --watch]
                   [--state-dir PATH] [--poll-seconds N]

setup previews missing AI labels; --apply adds them without replacing existing labels.
worker reads PATH/.factorykit/worker.json. Default: process one approved issue.
--dry-run lists queued issues without AI calls or GitHub writes.
--watch polls serially (default 120 seconds, minimum 30); Ctrl-C stops it.
State defaults to ~/.local/state/factorykit. Keep one state root per Mac.
Implementation/review require an existing Claude subscription login.
No automatic merge, deployment, daemon installation, or product update.
";

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(Error::Interrupted) => std::process::exit(130),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.is_empty()
        || args == ["--help"]
        || args == ["-h"]
        || (args.len() == 2
            && ["setup", "worker"].contains(&args[0].as_str())
            && ["--help", "-h"].contains(&args[1].as_str()))
    {
        print!("{HELP}");
        return Ok(0);
    }
    if args == ["--version"] {
        println!("factorykit {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    let options = Options::parse(&args)?;
    let runner = Runner::default();
    let stopped = runner.stop.clone();
    ctrlc::set_handler(move || stopped.store(true, Ordering::Relaxed))
        .map_err(|_| Error::blocked("Cannot install interruption handler."))?;
    let cwd = std::env::current_dir()?;
    let api = GitHub {
        runner: &runner,
        cwd: &cwd,
    };
    if args[0] == "setup" {
        let repository = Repository::parse(options.required("--repository")?)?;
        print_json(&github::setup(
            &api,
            &repository,
            options.flags.contains("--apply"),
        )?)?;
        return Ok(0);
    }
    let project = PathBuf::from(options.required("--project")?).canonicalize()?;
    let config = Config::load(&project.join(".factorykit/worker.json"))?;
    let root = match options.values.get("--state-dir") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| Error::blocked("Set --state-dir when HOME is unavailable."))?,
        )
        .join(".local/state/factorykit"),
    };
    let state = root.join(
        config
            .repository
            .as_str()
            .to_ascii_lowercase()
            .replace('/', "__"),
    );
    let _lock = state::lock(&state)?;
    let claude = Claude { runner: &runner };
    let dry_run = options.flags.contains("--dry-run");
    let watch = options.flags.contains("--watch");
    if !dry_run {
        claude.check_subscription(&project)?;
    }
    let worker = Worker {
        project: &project,
        state: &state,
        config: &config,
        queue: Queue {
            api: &api,
            repository: &config.repository,
        },
        agent: &claude,
        runner: &runner,
    };
    let poll: u64 = options
        .values
        .get("--poll-seconds")
        .map_or(Ok(120), |value| value.parse())
        .map_err(|_| Error::blocked("--poll-seconds must be an integer of at least 30."))?;
    if poll < 30 {
        return Err(Error::blocked("--poll-seconds must be at least 30."));
    }
    loop {
        let result = worker.once(dry_run)?;
        print_json(&result)?;
        let status = result["status"].as_str().unwrap_or_default();
        if !watch {
            return Ok(i32::from(matches!(status, "blocked" | "report-failed")));
        }
        if status == "report-failed" {
            return Err(Error::blocked(
                "GitHub reporting failed; stopped to prevent repeated attempts.",
            ));
        }
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(poll) {
            if runner.stop.load(Ordering::Relaxed) {
                return Err(Error::Interrupted);
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn print_json(value: &serde_json::Value) -> Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    serde_json::to_writer(&mut writer, value)?;
    writeln!(writer)?;
    writer.flush()?;
    Ok(())
}

struct Options {
    values: BTreeMap<String, String>,
    flags: BTreeSet<String>,
}
impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let (value_names, flag_names): (&[&str], &[&str]) = match args[0].as_str() {
            "setup" => (&["--repository"], &["--apply"]),
            "worker" => (
                &["--project", "--state-dir", "--poll-seconds"],
                &["--dry-run", "--watch"],
            ),
            _ => return Err(Error::blocked("Unknown command; run factorykit --help.")),
        };
        let mut options = Self {
            values: BTreeMap::new(),
            flags: BTreeSet::new(),
        };
        let mut rest = args[1..].iter();
        while let Some(name) = rest.next() {
            if value_names.contains(&name.as_str()) {
                let value = rest
                    .next()
                    .filter(|value| !value.starts_with("--"))
                    .ok_or_else(|| {
                        Error::blocked("Missing option value; run factorykit --help.")
                    })?;
                if options.values.insert(name.clone(), value.clone()).is_some() {
                    return Err(Error::blocked("Duplicate option."));
                }
            } else if !flag_names.contains(&name.as_str()) || !options.flags.insert(name.clone()) {
                return Err(Error::blocked(
                    "Unknown or duplicate option; run factorykit --help.",
                ));
            }
        }
        if options.flags.contains("--dry-run") && options.flags.contains("--watch") {
            return Err(Error::blocked("Choose --dry-run or --watch."));
        }
        Ok(options)
    }
    fn required(&self, name: &str) -> Result<&str> {
        self.values
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| Error::blocked(format!("Required option: {name}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_cli_cannot_accidentally_start_a_worker() {
        for args in [
            vec!["unknown"],
            vec!["worker", "--dry-run", "--watch"],
            vec!["setup", "--apply", "--watch"],
            vec!["worker", "--project"],
        ] {
            assert!(
                Options::parse(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err()
            );
        }
    }
}
