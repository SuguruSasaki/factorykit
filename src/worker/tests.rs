use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use super::*;
use crate::github::Api;

fn filler(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|n| format!("{prefix} 日本語 line {n} padding padding padding\n"))
        .collect()
}

/// Same shape as a real failure: failures in stdout, then a long build log in stderr.
fn long_stderr_log() -> String {
    let mut log = filler("out", 100);
    log.push_str("test fake::broken ... FAILED\n");
    log.push_str("thread 'fake::broken' panicked at src/fake.rs:1:1:\n");
    log.push_str("assertion failed: fake\n");
    log.push_str(&filler("out", 50));
    log.push_str("Validation results: fmt=FAIL clippy=FAIL test=FAIL\n");
    log.push_str("--- stderr ---\n");
    log.push_str(&filler("   Compiling fake", 400));
    log
}

#[test]
fn repair_excerpt_keeps_failure_before_long_stderr() {
    let log = long_stderr_log();
    assert!(log.chars().count() > 10_000);
    let (note, excerpt) = repair_excerpt(&log, 10_000);
    assert!(excerpt.contains("fake::broken ... FAILED"));
    assert!(excerpt.contains("panicked at src/fake.rs"));
    assert!(excerpt.contains("Validation results:"));
    assert!(excerpt.contains("Compiling fake 日本語 line 399"));
    assert!(note.contains(&format!("{} lines", log.lines().count())));
}

#[test]
fn repair_excerpt_keeps_fmt_clippy_and_test_failures() {
    let mut log = filler("out", 100);
    log.push_str("Diff in /fake/src/a.rs:1:\n-old\n+new\n");
    log.push_str(&filler("out", 100));
    log.push_str("error: fake clippy lint\n");
    log.push_str(&filler("out", 100));
    log.push_str("test fake::broken ... FAILED\n");
    log.push_str(&filler("out", 100));
    log.push_str("--- stderr ---\n");
    log.push_str(&filler("   Compiling fake", 400));
    let (_, excerpt) = repair_excerpt(&log, 10_000);
    assert!(excerpt.contains("Diff in /fake/src/a.rs"));
    assert!(excerpt.contains("error: fake clippy lint"));
    assert!(excerpt.contains("fake::broken ... FAILED"));
}

#[test]
fn repair_excerpt_without_markers_is_the_tail() {
    let log = filler("plain", 1000);
    let (_, excerpt) = repair_excerpt(&log, 10_000);
    let expected: Vec<char> = log.chars().collect();
    let expected: String = expected[expected.len() - 10_000..].iter().collect();
    assert_eq!(excerpt, expected);
}

#[test]
fn repair_excerpt_respects_limit_and_marks_omissions() {
    let log = long_stderr_log();
    for limit in [0, 50, 500, 3_000, 10_000] {
        let (_, excerpt) = repair_excerpt(&log, limit);
        assert!(excerpt.chars().count() <= limit, "limit {limit}");
    }
    let (_, excerpt) = repair_excerpt(&log, 10_000);
    assert!(excerpt.contains("lines 1-"));
    assert!(excerpt.contains(" omitted ...]"));
}

#[test]
fn repair_excerpt_short_log_is_unchanged() {
    let (_, excerpt) = repair_excerpt("a FAILED\n", 10_000);
    assert_eq!(excerpt, "a FAILED\n");
}

fn issue_value() -> Value {
    json!({"number":7,"title":"Add greeting","body":"### 作業種別\nfeature\n### 目的\nGreet the user.\n### 対象範囲\nhello.txt only.\n### 期待する動作\nHello is present.\n### 完了条件\nThe file contains Hello.\n### 検証方法\nRead hello.txt.\n","state":"open","labels":[{"name":"ai-in-progress"}],"user":{"login":"owner","type":"User"}})
}

struct FakeApi {
    issue: RefCell<Value>,
    writes: RefCell<Vec<(String, Value)>>,
    queued: Cell<bool>,
    existing_pr: Cell<bool>,
    approver_kind: RefCell<String>,
    permission: RefCell<String>,
    fail_comments: Cell<bool>,
    fail_pr_label: Cell<bool>,
    /// Other issues by number; an absent number is a GitHub 404.
    others: RefCell<std::collections::BTreeMap<u64, Value>>,
}
impl Default for FakeApi {
    fn default() -> Self {
        Self {
            others: RefCell::new(Default::default()),
            issue: RefCell::new(issue_value()),
            writes: RefCell::new(vec![]),
            queued: Cell::new(false),
            existing_pr: Cell::new(false),
            approver_kind: RefCell::new("User".to_owned()),
            permission: RefCell::new("write".to_owned()),
            fail_comments: Cell::new(false),
            fail_pr_label: Cell::new(false),
        }
    }
}
impl Api for FakeApi {
    fn request(
        &self,
        method: &str,
        endpoint: &str,
        payload: Option<Value>,
        _pages: bool,
    ) -> Result<Value> {
        if method == "GET" {
            if endpoint == "user" {
                return Ok(json!({"login":"owner","id":42}));
            }
            if endpoint.contains("issues?") {
                return Ok(if self.queued.get() {
                    json!([[*self.issue.borrow()]])
                } else {
                    json!([[]])
                });
            }
            if endpoint.contains("/events?") {
                return Ok(
                    json!([[{"event":"labeled","label":{"name":"ai-ready"},"actor":{"login":"approver","type":*self.approver_kind.borrow()}}]]),
                );
            }
            if endpoint.contains("/permission") {
                return Ok(json!({"permission":*self.permission.borrow()}));
            }
            if endpoint.contains("pulls?") {
                return Ok(if self.existing_pr.get() {
                    json!([{"number":9}])
                } else {
                    json!([])
                });
            }
            if let Some(number) = endpoint
                .strip_prefix("repos/owner/repo/issues/")
                .and_then(|n| n.parse::<u64>().ok())
                .filter(|n| *n != 7)
            {
                return self
                    .others
                    .borrow()
                    .get(&number)
                    .cloned()
                    .ok_or_else(|| Error::blocked("Command failed: gh"));
            }
            return Ok(self.issue.borrow().clone());
        }
        let payload = payload.unwrap_or(Value::Null);
        self.writes
            .borrow_mut()
            .push((endpoint.to_owned(), payload.clone()));
        if endpoint.ends_with("/comments") && self.fail_comments.get() {
            return Err(Error::blocked("Simulated reporting outage."));
        }
        if endpoint.ends_with("issues/9/labels") && self.fail_pr_label.get() {
            return Err(Error::blocked("Simulated label failure."));
        }
        if endpoint.ends_with("issues/7/labels") {
            self.issue.borrow_mut()["labels"]
                .as_array_mut()
                .expect("labels")
                .push(json!({"name":payload["labels"][0]}));
        } else if method == "DELETE" {
            let name = endpoint.rsplit('/').next().expect("name");
            self.issue.borrow_mut()["labels"]
                .as_array_mut()
                .expect("labels")
                .retain(|label| label["name"] != name);
        }
        if endpoint.ends_with("/pulls") {
            return Ok(
                json!({"html_url":"https://github.com/owner/repo/pull/9","number":9,"node_id":"PR_node"}),
            );
        }
        Ok(Value::Null)
    }
}

#[derive(Clone, Copy)]
enum Behavior {
    Good,
    Protected,
    RenameProtected,
    Repair,
    AlwaysFail,
    MutateReview,
    RejectReview,
}
struct FakeAgent {
    behavior: Behavior,
    calls: Cell<usize>,
}
impl Agent for FakeAgent {
    fn ask(&self, task: Task<'_>) -> Result<Value> {
        self.calls.set(self.calls.get() + 1);
        if matches!(task.role, Role::Review) {
            if matches!(self.behavior, Behavior::MutateReview) {
                fs::write(task.work.join("hello.txt"), "tampered")?;
            }
            return Ok(
                json!({"verdict":if matches!(self.behavior,Behavior::RejectReview) {"fail"} else {"pass"},"summary":"Independent review","findings":[]}),
            );
        }
        match self.behavior {
            Behavior::Protected => fs::write(task.work.join("AGENTS.md"), "changed")?,
            Behavior::RenameProtected => {
                fs::rename(task.work.join("AGENTS.md"), task.work.join("moved.md"))?
            }
            Behavior::AlwaysFail => fs::write(task.work.join("hello.txt"), "wrong")?,
            Behavior::Repair if matches!(task.role, Role::Implement) => {
                fs::write(task.work.join("hello.txt"), "wrong")?
            }
            Behavior::Repair => {
                assert!(task.prompt.contains("Failed check: Greeting contract"));
                assert!(task.prompt.contains("argv:") && task.prompt.contains("Hello"));
                assert!(task.prompt.contains("\"status\":\"failed\""));
                fs::write(task.work.join("hello.txt"), "Hello\n")?;
            }
            _ => fs::write(task.work.join("hello.txt"), "Hello\n")?,
        }
        Ok(
            json!({"status":"ready","summary":"Add a greeting","manual_checks":["Read the greeting as a user."]}),
        )
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    project: PathBuf,
    work: PathBuf,
    job: PathBuf,
    origin: PathBuf,
    config: Config,
    runner: Runner,
    api: FakeApi,
    agent: FakeAgent,
}
impl Fixture {
    fn new(behavior: Behavior) -> Self {
        let root = tempfile::tempdir().expect("temp");
        let project = root.path().join("project");
        let work = root.path().join("work");
        let job = root.path().join("job");
        let origin = root.path().join("origin.git");
        fs::create_dir(&project).expect("project");
        fs::create_dir(&job).expect("job");
        let runner = Runner::default();
        let git = |cwd: &Path, args: &[&str]| {
            runner
                .run(Request::new("git", cwd, args.iter().copied()))
                .expect("git fixture")
        };
        git(&project, &["init", "--quiet", "-b", "main"]);
        fs::write(project.join("AGENTS.md"), "Keep changes small.\n").expect("instructions");
        git(&project, &["add", "."]);
        git(
            &project,
            &[
                "-c",
                "user.name=owner",
                "-c",
                "user.email=42+owner@users.noreply.github.com",
                "commit",
                "--quiet",
                "-m",
                "Initial",
            ],
        );
        git(
            root.path(),
            &[
                "clone",
                "--quiet",
                "--bare",
                project.to_str().expect("path"),
                origin.to_str().expect("path"),
            ],
        );
        git(
            root.path(),
            &[
                "clone",
                "--quiet",
                origin.to_str().expect("path"),
                work.to_str().expect("path"),
            ],
        );
        git(&work, &["switch", "--quiet", "-c", "ai/issue-7"]);
        let config = serde_json::from_value(json!({"repository":"owner/repo","checks":[{"name":"Greeting contract","argv":["sh","-c","test \"$(cat hello.txt)\" = Hello"]}]})).expect("config");
        Self {
            _root: root,
            project,
            work,
            job,
            origin,
            config,
            runner,
            api: FakeApi::default(),
            agent: FakeAgent {
                behavior,
                calls: Cell::new(0),
            },
        }
    }
    fn worker(&self) -> Worker<'_> {
        Worker {
            project: &self.project,
            state: &self.job,
            config: &self.config,
            queue: Queue {
                api: &self.api,
                repository: &self.config.repository,
            },
            agent: &self.agent,
            runner: &self.runner,
        }
    }
    fn execute(&self) -> Result<Value> {
        let issue = serde_json::from_value(issue_value()).expect("issue");
        self.worker().execute(
            &issue,
            TaskKind::Feature,
            &self.job,
            &self.work,
            "ai/issue-7",
            &mut json!({}),
        )
    }
    fn published(&self) -> bool {
        self.api
            .writes
            .borrow()
            .iter()
            .any(|(endpoint, _)| endpoint.ends_with("/pulls"))
    }
}

#[test]
fn successful_run_checks_reviews_and_pushes_only_its_branch() {
    let fixture = Fixture::new(Behavior::Good);
    let before = fs::read_to_string(fixture.project.join("AGENTS.md")).expect("original");
    assert_eq!(fixture.execute().expect("run")["status"], "pr-created");
    assert_eq!(fixture.agent.calls.get(), 2);
    assert_eq!(
        fs::read_to_string(fixture.project.join("AGENTS.md")).expect("original"),
        before
    );
    assert!(!fixture.project.join("hello.txt").exists());
    let remote = fixture
        .runner
        .run(Request::new(
            "git",
            &fixture.origin,
            ["show", "refs/heads/ai/issue-7:hello.txt"],
        ))
        .expect("remote branch");
    assert_eq!(remote, "Hello\n");
    let writes = fixture.api.writes.borrow();
    let (_, pr) = writes
        .iter()
        .find(|(endpoint, _)| endpoint.ends_with("/pulls"))
        .expect("PR");
    assert_eq!(pr["draft"], true);
    assert!(
        pr["body"]
            .as_str()
            .expect("body")
            .contains("Greeting contract | verified")
    );
    assert!(
        pr["body"]
            .as_str()
            .expect("body")
            .contains("Read the greeting as a user.")
    );
    assert_eq!(
        fixture.api.issue.borrow()["labels"],
        json!([{"name":"ai-review"}])
    );
}

fn count_writes(fixture: &Fixture, endpoint: &str) -> usize {
    fixture
        .api
        .writes
        .borrow()
        .iter()
        .filter(|(written, _)| written.ends_with(endpoint))
        .count()
}

#[test]
fn published_pr_is_marked_ready_and_labeled_once() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.execute().expect("run");
    assert_eq!(count_writes(&fixture, "graphql"), 1);
    assert_eq!(count_writes(&fixture, "issues/9/labels"), 1);
    let writes = fixture.api.writes.borrow();
    let (_, label) = writes
        .iter()
        .find(|(endpoint, _)| endpoint.ends_with("issues/9/labels"))
        .expect("label");
    assert_eq!(label["labels"], json!(["human-merge"]));
    let (_, ready) = writes
        .iter()
        .find(|(endpoint, _)| endpoint.ends_with("graphql"))
        .expect("ready");
    assert_eq!(ready["variables"]["id"], "PR_node");
}

#[test]
fn label_failure_leaves_draft_and_explains_on_the_issue() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.api.fail_pr_label.set(true);
    assert_eq!(fixture.execute().expect("run")["status"], "pr-created");
    assert_eq!(count_writes(&fixture, "graphql"), 0);
    assert_eq!(
        fixture.api.issue.borrow()["labels"],
        json!([{"name":"ai-review"}])
    );
    let writes = fixture.api.writes.borrow();
    let (_, comment) = writes
        .iter()
        .rfind(|(endpoint, _)| endpoint.ends_with("issues/7/comments"))
        .expect("comment");
    let text = comment["body"].as_str().expect("body");
    assert!(text.contains("stays Draft") && text.contains("Simulated label failure"));
}

#[test]
fn failed_checks_never_label_a_pr() {
    let fixture = Fixture::new(Behavior::AlwaysFail);
    assert!(fixture.execute().is_err());
    assert_eq!(count_writes(&fixture, "graphql"), 0);
    assert_eq!(count_writes(&fixture, "issues/9/labels"), 0);
}

#[test]
fn failed_check_gets_one_repair_then_independent_review() {
    let fixture = Fixture::new(Behavior::Repair);
    assert!(fixture.execute().is_ok());
    assert_eq!(fixture.agent.calls.get(), 3);
    assert!(fixture.job.join("checks-1.json").exists());
    assert!(fixture.job.join("checks-2.json").exists());
}

#[test]
fn recurring_failure_stops_after_one_repair_without_publication() {
    let fixture = Fixture::new(Behavior::AlwaysFail);
    assert!(fixture.execute().is_err());
    assert_eq!(fixture.agent.calls.get(), 2);
    assert!(!fixture.published());
}

#[test]
fn protected_edits_and_renames_stop_before_review() {
    for behavior in [Behavior::Protected, Behavior::RenameProtected] {
        let fixture = Fixture::new(behavior);
        assert!(fixture.execute().is_err());
        assert_eq!(fixture.agent.calls.get(), 1);
        assert!(!fixture.published());
    }
}

#[test]
fn rejected_or_mutating_review_cannot_publish() {
    for behavior in [Behavior::RejectReview, Behavior::MutateReview] {
        let fixture = Fixture::new(behavior);
        assert!(fixture.execute().is_err());
        assert!(!fixture.published());
    }
}

#[test]
fn revoked_or_changed_issue_stops_before_publication() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.api.issue.borrow_mut()["body"] = json!("Changed scope");
    assert!(fixture.execute().is_err());
    assert!(!fixture.published());
}

#[test]
fn uncovered_files_cannot_be_reported_as_verified() {
    let mut fixture = Fixture::new(Behavior::Good);
    fixture.config.checks[0].paths = vec!["src/**".to_owned()];
    assert!(fixture.execute().is_err());
    assert_eq!(
        fixture.agent.calls.get(),
        1,
        "uncovered paths need configuration, not an AI repair"
    );
    assert!(!fixture.published());
}

#[test]
fn empty_queue_and_preview_use_no_agent_and_write_nothing() {
    let fixture = Fixture::new(Behavior::Good);
    assert_eq!(
        fixture.worker().once(false).expect("idle")["status"],
        "idle"
    );
    fixture.api.queued.set(true);
    fixture.api.issue.borrow_mut()["labels"] = json!([{"name":"ai-ready"}]);
    assert_eq!(
        fixture.worker().once(true).expect("preview")["ready"]
            .as_array()
            .expect("ready")
            .len(),
        1
    );
    assert_eq!(fixture.agent.calls.get(), 0);
    assert!(fixture.api.writes.borrow().is_empty());
}

#[test]
fn bots_and_read_only_approvers_cannot_trigger_work() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.api.queued.set(true);
    fixture.api.issue.borrow_mut()["labels"] = json!([{"name":"ai-ready"}]);
    *fixture.api.approver_kind.borrow_mut() = "Bot".to_owned();
    assert_eq!(
        fixture.worker().once(false).expect("unapproved")["status"],
        "unapproved"
    );
    *fixture.api.approver_kind.borrow_mut() = "User".to_owned();
    *fixture.api.permission.borrow_mut() = "read".to_owned();
    assert_eq!(
        fixture.worker().once(false).expect("unapproved")["status"],
        "unapproved"
    );
    assert_eq!(fixture.agent.calls.get(), 0);
}

#[test]
fn existing_pr_moves_to_review_without_duplicate_agent_work() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.api.queued.set(true);
    fixture.api.existing_pr.set(true);
    fixture.api.issue.borrow_mut()["labels"] = json!([{"name":"ai-ready"},{"name":"product"}]);
    assert_eq!(
        fixture.worker().once(false).expect("existing")["status"],
        "existing-pr"
    );
    assert_eq!(fixture.agent.calls.get(), 0);
    assert_eq!(
        fixture.api.issue.borrow()["labels"],
        json!([{"name":"product"},{"name":"ai-review"}])
    );
}

#[test]
fn invalid_task_is_blocked_and_reporting_failure_is_explicit() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.api.queued.set(true);
    fixture.api.issue.borrow_mut()["labels"] = json!([{"name":"ai-ready"}]);
    fixture.api.issue.borrow_mut()["body"] = json!("Missing acceptance");
    fixture.api.fail_comments.set(true);
    assert_eq!(
        fixture.worker().once(false).expect("failure")["status"],
        "report-failed"
    );
    assert_eq!(fixture.agent.calls.get(), 0);
    assert_eq!(
        fixture.api.issue.borrow()["labels"],
        json!([{"name":"ai-blocked"}])
    );
    let run = fs::read_dir(&fixture.job)
        .expect("runs")
        .next()
        .expect("one run")
        .expect("entry")
        .path();
    let receipt: Value =
        serde_json::from_slice(&fs::read(run.join("receipt.json")).expect("receipt"))
            .expect("JSON");
    assert!(
        receipt["run"].is_string(),
        "run IDs must be readable JSON strings"
    );
}

fn other(number: u64, state: &str, reason: Option<&str>, prerequisites: &str) -> Value {
    json!({"number":number,"title":"other","body":format!("### 前提\n{prerequisites}\n"),
        "state":state,"state_reason":reason,"labels":[],"user":{"login":"owner","type":"User"}})
}
/// A queued, approved issue #7 whose 前提 section is `text`.
fn queued_with(fixture: &Fixture, text: &str) {
    fixture.api.queued.set(true);
    let mut issue = fixture.api.issue.borrow_mut();
    issue["labels"] = json!([{"name":"ai-ready"}]);
    let body = issue["body"].as_str().expect("body").to_owned();
    issue["body"] = json!(format!("{body}### 前提\n{text}\n"));
}
fn labels_posted(fixture: &Fixture) -> Vec<Value> {
    fixture
        .api
        .writes
        .borrow()
        .iter()
        .filter(|(endpoint, _)| endpoint.ends_with("/labels"))
        .map(|(_, payload)| payload["labels"][0].clone())
        .collect()
}

#[test]
fn open_prerequisite_skips_without_label_change_or_comment_until_completed() {
    let fixture = Fixture::new(Behavior::Good);
    fixture.api.existing_pr.set(true); // a picked issue stops here, offline
    queued_with(&fixture, "#12, #13");
    {
        let mut others = fixture.api.others.borrow_mut();
        others.insert(12, other(12, "closed", Some("completed"), ""));
        others.insert(13, other(13, "open", None, ""));
    }
    let result = fixture.worker().once(false).expect("waiting");
    assert_eq!(result["status"], "waiting");
    assert_eq!(result["waiting"], json!([{"issue":7,"waiting_for":[13]}]));
    assert!(fixture.api.writes.borrow().is_empty(), "no write");
    assert_eq!(
        fixture.api.issue.borrow()["labels"],
        json!([{"name":"ai-ready"}])
    );
    // Dry-run separates what can start from what waits, and names the number.
    let preview = fixture.worker().once(true).expect("preview");
    assert_eq!(preview["ready"], json!([]));
    assert_eq!(preview["waiting"][0]["waiting_for"], json!([13]));
    assert!(fixture.api.writes.borrow().is_empty());
    // Next patrol after the prerequisite completed.
    fixture
        .api
        .others
        .borrow_mut()
        .insert(13, other(13, "closed", Some("completed"), ""));
    assert_eq!(
        fixture.worker().once(true).expect("preview")["ready"][0]["number"],
        7
    );
    assert_eq!(
        fixture.worker().once(false).expect("picked")["status"],
        "existing-pr"
    );
    assert_eq!(fixture.agent.calls.get(), 0);
}

#[test]
fn issue_without_or_with_completed_prerequisites_is_picked_as_before() {
    for text in ["", "#12"] {
        let fixture = Fixture::new(Behavior::Good);
        fixture.api.existing_pr.set(true);
        queued_with(&fixture, text);
        fixture
            .api
            .others
            .borrow_mut()
            .insert(12, other(12, "closed", Some("completed"), ""));
        assert_eq!(
            fixture.worker().once(false).expect("picked")["status"],
            "existing-pr",
            "{text:?}"
        );
    }
}

#[test]
fn unsatisfiable_prerequisites_block_the_issue_with_a_reason() {
    let not_planned = other(12, "closed", Some("not_planned"), "");
    let no_reason = other(12, "closed", None, "");
    let (first, second) = (
        other(12, "open", None, "#13"),
        other(13, "open", None, "#7"),
    );
    type Case<'a> = (&'a str, Vec<(u64, Value)>, &'a str);
    let cases: [Case; 6] = [
        ("#12", vec![(12, not_planned)], "#12"),
        ("#99", vec![], "#99"),
        ("#7", vec![], "itself"),
        ("owner/other#3", vec![], "owner/other#3"),
        ("#12", vec![(12, first), (13, second)], "cycle"),
        ("#12", vec![(12, no_reason)], "#12"),
    ];
    for (text, others, expected) in cases {
        let fixture = Fixture::new(Behavior::Good);
        queued_with(&fixture, text);
        fixture.api.others.borrow_mut().extend(others);
        let result = fixture.worker().once(false).expect("blocked");
        assert_eq!(result["status"], "waiting", "{text}");
        assert_eq!(result["blocked"][0]["issue"], 7, "{text}");
        assert_eq!(
            fixture.api.issue.borrow()["labels"],
            json!([{"name":"ai-blocked"}]),
            "{text}"
        );
        assert_eq!(labels_posted(&fixture), vec![json!("ai-blocked")], "{text}");
        let writes = fixture.api.writes.borrow();
        let comments: Vec<_> = writes
            .iter()
            .filter(|(endpoint, _)| endpoint.ends_with("/comments"))
            .collect();
        assert_eq!(comments.len(), 1, "{text}");
        let body = comments[0].1["body"].as_str().expect("body");
        assert!(body.contains(expected), "{text}");
        assert_eq!(fixture.agent.calls.get(), 0);
    }
}

#[test]
fn product_feature_map_can_change_without_modifying_validator() {
    let fixture = Fixture::new(Behavior::Good);
    let file = fixture
        .work
        .join(".agents/skills/verify-product/references/features.md");
    fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    fs::write(file, "Updated map\n").expect("map");
    assert!(fixture.worker().stage_and_guard(&fixture.work).is_ok());
}
