use std::path::Path;

use serde_json::{Value, json};

use crate::domain::{Issue, Repository};
use crate::error::{Error, Result};
use crate::process::{Request, Runner};

pub(crate) const STATES: [&str; 4] = ["ai-ready", "ai-in-progress", "ai-blocked", "ai-review"];

/// PR label: factory work and checks are done; a person merges. Not approval to update the app.
pub(crate) const HUMAN_MERGE: &str = "human-merge";

/// Whether an issue's `前提` issues allow starting it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gate {
    Ready,
    /// Open prerequisite numbers; skip without changing labels.
    Waiting(Vec<u64>),
    /// The prerequisites can never be satisfied as written.
    Blocked(String),
}

/// The transport boundary allows queue and label tests without GitHub writes.
pub(crate) trait Api {
    fn request(
        &self,
        method: &str,
        endpoint: &str,
        payload: Option<Value>,
        pages: bool,
    ) -> Result<Value>;
}

pub(crate) struct GitHub<'a> {
    pub runner: &'a Runner,
    pub cwd: &'a Path,
}

impl Api for GitHub<'_> {
    fn request(
        &self,
        method: &str,
        endpoint: &str,
        payload: Option<Value>,
        pages: bool,
    ) -> Result<Value> {
        let mut args = vec!["api", "--method", method, endpoint];
        if pages {
            args.extend(["--paginate", "--slurp"]);
        }
        if payload.is_some() {
            args.extend(["--input", "-"]);
        }
        let mut request = Request::new("gh", self.cwd, args);
        request.input = payload.map(|value| value.to_string());
        let output = self.runner.run(request)?;
        if output.trim().is_empty() {
            Ok(Value::Null)
        } else {
            Ok(serde_json::from_str(&output)?)
        }
    }
}

pub(crate) struct Queue<'a> {
    pub api: &'a dyn Api,
    pub repository: &'a Repository,
}

impl Queue<'_> {
    pub(crate) fn request(
        &self,
        method: &str,
        path: &str,
        payload: Option<Value>,
        pages: bool,
    ) -> Result<Value> {
        self.api.request(
            method,
            &format!("repos/{}/{path}", self.repository.as_str()),
            payload,
            pages,
        )
    }
    pub(crate) fn ready(&self) -> Result<Vec<Issue>> {
        let pages = self.request(
            "GET",
            "issues?state=open&labels=ai-ready&sort=created&direction=asc&per_page=100",
            None,
            true,
        )?;
        let mut result = Vec::new();
        for page in array(&pages)? {
            for value in array(page)? {
                if value.get("pull_request").is_none() {
                    let issue: Issue = serde_json::from_value(value.clone())?;
                    if issue.eligible() {
                        result.push(issue);
                    }
                }
            }
        }
        Ok(result)
    }
    pub(crate) fn issue(&self, number: u64) -> Result<Issue> {
        Ok(serde_json::from_value(self.request(
            "GET",
            &format!("issues/{number}"),
            None,
            false,
        )?)?)
    }
    /// An issue that GitHub does not know is `None`. If the repository itself
    /// cannot be read, the failure is an outage and propagates instead.
    fn lookup(&self, number: u64) -> Result<Option<Issue>> {
        match self.issue(number) {
            Ok(issue) => Ok(Some(issue)),
            Err(Error::Blocked(_)) => {
                self.api.request(
                    "GET",
                    &format!("repos/{}", self.repository.as_str()),
                    None,
                    false,
                )?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
    /// Decide whether the issue's `前提` issues allow starting it. Only reads.
    pub(crate) fn gate(&self, issue: &Issue) -> Result<Gate> {
        let prerequisites = match issue.prerequisites() {
            Ok(numbers) => numbers,
            Err(error) => return Ok(Gate::Blocked(error.public_message().to_owned())),
        };
        let mut waiting = Vec::new();
        let mut open = Vec::new();
        for number in prerequisites {
            let Some(found) = self.lookup(number)? else {
                return Ok(Gate::Blocked(format!(
                    "Prerequisite #{number} does not exist in this repository."
                )));
            };
            match (found.state.as_str(), found.state_reason.as_deref()) {
                ("open", _) => {
                    waiting.push(number);
                    open.push(found);
                }
                ("closed", Some("completed")) => {}
                _ => {
                    return Ok(Gate::Blocked(format!(
                        "Prerequisite #{number} was closed without being completed (not planned); decide whether to remove it from 前提, then requeue."
                    )));
                }
            }
        }
        if waiting.is_empty() {
            return Ok(Gate::Ready);
        }
        let mut path = vec![issue.number];
        let mut seen = std::collections::BTreeSet::new();
        for found in &open {
            if let Some(reason) = self.cycle(found, &mut path, &mut seen)? {
                return Ok(Gate::Blocked(reason));
            }
        }
        Ok(Gate::Waiting(waiting))
    }
    /// Follow open prerequisites looking for a way back to a path member.
    fn cycle(
        &self,
        issue: &Issue,
        path: &mut Vec<u64>,
        seen: &mut std::collections::BTreeSet<u64>,
    ) -> Result<Option<String>> {
        if !seen.insert(issue.number) {
            return Ok(None);
        }
        if seen.len() > 50 {
            return Ok(Some("Prerequisite chain is too long to check.".to_owned()));
        }
        // Another issue's malformed list blocks that issue, not this one.
        let Ok(next) = issue.prerequisites() else {
            return Ok(None);
        };
        path.push(issue.number);
        for number in next {
            if path.contains(&number) {
                path.push(number);
                let reason = format!(
                    "Prerequisites form a cycle: {}.",
                    path.iter()
                        .map(|n| format!("#{n}"))
                        .collect::<Vec<_>>()
                        .join(" -> ")
                );
                return Ok(Some(reason));
            }
            if let Some(found) = self.lookup(number)?.filter(|found| found.state == "open")
                && let Some(reason) = self.cycle(&found, path, seen)?
            {
                return Ok(Some(reason));
            }
        }
        path.pop();
        Ok(None)
    }
    pub(crate) fn trusted(&self, issue: &Issue) -> Result<bool> {
        let pages = self.request(
            "GET",
            &format!("issues/{}/events?per_page=100", issue.number),
            None,
            true,
        )?;
        let mut approver = None;
        for page in array(&pages)? {
            for event in array(page)? {
                if event["event"] == "labeled"
                    && event["label"]["name"]
                        .as_str()
                        .is_some_and(|name| name.eq_ignore_ascii_case("ai-ready"))
                {
                    approver = event.get("actor");
                }
            }
        }
        let Some(approver) = approver else {
            return Ok(false);
        };
        if issue.user.kind != "User" || approver["type"] != "User" {
            return Ok(false);
        }
        let Some(login) = approver["login"].as_str() else {
            return Ok(false);
        };
        for login in [issue.user.login.as_str(), login] {
            let permission = self.request(
                "GET",
                &format!("collaborators/{}/permission", encode(login)),
                None,
                false,
            )?;
            if !matches!(
                permission["permission"].as_str(),
                Some("admin" | "maintain" | "write")
            ) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(crate) fn transition(&self, number: u64, target: &str) -> Result<()> {
        if !STATES.contains(&target) {
            return Err(Error::blocked("Unknown queue state."));
        }
        self.request(
            "POST",
            &format!("issues/{number}/labels"),
            Some(json!({"labels":[target]})),
            false,
        )?;
        for label in self.issue(number)?.labels {
            if STATES
                .iter()
                .any(|state| label.name.eq_ignore_ascii_case(state))
                && !label.name.eq_ignore_ascii_case(target)
            {
                self.request(
                    "DELETE",
                    &format!("issues/{number}/labels/{}", encode(&label.name)),
                    None,
                    false,
                )?;
            }
        }
        Ok(())
    }
    pub(crate) fn comment(&self, number: u64, body: &str) -> Result<()> {
        self.request(
            "POST",
            &format!("issues/{number}/comments"),
            Some(json!({"body":body})),
            false,
        )?;
        Ok(())
    }
    pub(crate) fn existing_pr(&self, branch: &str) -> Result<bool> {
        let head = encode(&format!("{}:{branch}", self.repository.owner()));
        Ok(
            !array(&self.request("GET", &format!("pulls?state=open&head={head}"), None, false)?)?
                .is_empty(),
        )
    }
    pub(crate) fn create_pr(
        &self,
        branch: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<PullRequest> {
        let value = self.request(
            "POST",
            "pulls",
            Some(json!({"head":branch,"base":base,"title":title,"body":body,"draft":true})),
            false,
        )?;
        Ok(PullRequest {
            url: string(&value, "html_url")?.to_owned(),
            number: value["number"]
                .as_u64()
                .ok_or_else(|| Error::blocked("Missing GitHub response field."))?,
            node_id: string(&value, "node_id")?.to_owned(),
        })
    }
    /// Mark a Draft PR as awaiting the human merge. The label goes first so a
    /// failure leaves the PR in Draft; if switching to Ready then fails, the
    /// label is withdrawn on a best-effort basis.
    pub(crate) fn publish_pr(&self, pr: &PullRequest) -> Result<()> {
        self.request(
            "POST",
            &format!("issues/{}/labels", pr.number),
            Some(json!({"labels":[HUMAN_MERGE]})),
            false,
        )?;
        let ready = self
            .api
            .request(
                "POST",
                "graphql",
                Some(json!({
                    "query":"mutation($id: ID!) { markPullRequestReadyForReview(input: {pullRequestId: $id}) { pullRequest { isDraft } } }",
                    "variables":{"id":pr.node_id},
                })),
                false,
            )
            .and_then(|value| {
                if value.get("errors").is_some_and(|errors| !errors.is_null()) {
                    Err(Error::blocked(
                        "GitHub refused to mark the pull request ready for review.",
                    ))
                } else {
                    Ok(())
                }
            });
        if let Err(error) = ready {
            let _ = self.request(
                "DELETE",
                &format!("issues/{}/labels/{HUMAN_MERGE}", pr.number),
                None,
                false,
            );
            return Err(error);
        }
        Ok(())
    }
}

/// A created pull request: the URL for people, the number for REST, the node ID for GraphQL.
pub(crate) struct PullRequest {
    pub url: String,
    pub number: u64,
    pub node_id: String,
}

pub(crate) fn array(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::blocked("Expected a GitHub array response."))
}
pub(crate) fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value[name]
        .as_str()
        .ok_or_else(|| Error::blocked("Missing GitHub response field."))
}
fn encode(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub(crate) fn setup(api: &dyn Api, repository: &Repository, apply: bool) -> Result<Value> {
    let queue = Queue { api, repository };
    let repo = api.request(
        "GET",
        &format!("repos/{}", repository.as_str()),
        None,
        false,
    )?;
    if repo["has_issues"] != true {
        return Err(Error::blocked("Enable Issues before preparing labels."));
    }
    let definitions: Vec<Value> = serde_json::from_str(include_str!(
        "../skills/factorykit/assets/issue-labels.json"
    ))?;
    let label_names = || -> Result<Vec<String>> {
        let pages = queue.request("GET", "labels?per_page=100", None, true)?;
        let mut names = Vec::new();
        for page in array(&pages)? {
            for label in array(page)? {
                names.push(string(label, "name")?.to_ascii_lowercase());
            }
        }
        Ok(names)
    };
    let existing = label_names()?;
    let mut missing = Vec::new();
    let mut preserved = Vec::new();
    for label in &definitions {
        let name = string(label, "name")?;
        if existing.contains(&name.to_ascii_lowercase()) {
            preserved.push(name);
        } else {
            missing.push(name);
        }
    }
    let mut created = Vec::new();
    if apply && !missing.is_empty() {
        if !["push", "maintain", "admin"]
            .iter()
            .any(|key| repo["permissions"][key] == true)
        {
            return Err(Error::blocked(
                "Write permission is required to prepare labels.",
            ));
        }
        for label in &definitions {
            let name = string(label, "name")?;
            if missing.contains(&name) {
                queue.request("POST", "labels", Some(label.clone()), false)?;
                created.push(name);
            }
        }
    }
    if apply {
        let names = label_names()?;
        if definitions.iter().any(|label| {
            label["name"]
                .as_str()
                .is_none_or(|name| !names.contains(&name.to_ascii_lowercase()))
        }) {
            return Err(Error::blocked(
                "Label setup incomplete; rerun to add missing labels.",
            ));
        }
    }
    Ok(
        json!({"repository":repo["full_name"],"mode":if apply {"apply"} else {"dry-run"},"missing":missing,"preserved":preserved,"created":created}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct Labels {
        labels: RefCell<Vec<Value>>,
        writes: RefCell<Vec<String>>,
        write: bool,
    }
    impl Api for Labels {
        fn request(
            &self,
            method: &str,
            endpoint: &str,
            payload: Option<Value>,
            _pages: bool,
        ) -> Result<Value> {
            if method == "POST" {
                self.writes.borrow_mut().push(endpoint.to_owned());
                self.labels.borrow_mut().push(payload.expect("payload"));
                return Ok(Value::Null);
            }
            if endpoint.ends_with("?per_page=100") {
                return Ok(json!([*self.labels.borrow()]));
            }
            Ok(
                json!({"full_name":"owner/repo","has_issues":true,"permissions":{"push":self.write}}),
            )
        }
    }
    #[test]
    fn setup_is_add_only_case_insensitive_and_idempotent() {
        let original = json!({"name":"AI-READY","color":"ffffff","description":"custom"});
        let api = Labels {
            labels: RefCell::new(vec![original.clone()]),
            writes: RefCell::new(vec![]),
            write: true,
        };
        let repo = Repository::parse("owner/repo").expect("repo");
        assert_eq!(
            setup(&api, &repo, false).expect("preview")["missing"]
                .as_array()
                .expect("array")
                .len(),
            4
        );
        assert!(api.writes.borrow().is_empty());
        let applied = setup(&api, &repo, true).expect("apply");
        assert_eq!(applied["created"].as_array().expect("array").len(), 4);
        assert!(
            applied["created"]
                .as_array()
                .expect("array")
                .contains(&json!("human-merge"))
        );
        assert_eq!(api.labels.borrow()[0], original);
        assert_eq!(
            setup(&api, &repo, true).expect("repeat")["created"],
            json!([])
        );
        assert_eq!(api.writes.borrow().len(), 4);
    }
    #[test]
    fn setup_rejects_missing_write_permission_without_writes() {
        let api = Labels {
            labels: RefCell::new(vec![]),
            writes: RefCell::new(vec![]),
            write: false,
        };
        assert!(setup(&api, &Repository::parse("owner/repo").expect("repo"), true).is_err());
        assert!(api.writes.borrow().is_empty());
    }
}
