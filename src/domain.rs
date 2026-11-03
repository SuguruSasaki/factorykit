use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub(crate) struct Repository(String);

impl Repository {
    pub(crate) fn parse(text: &str) -> Result<Self> {
        let parts: Vec<_> = text.split('/').collect();
        let valid = parts.len() == 2
            && parts.iter().all(|part| {
                !part.is_empty()
                    && !matches!(*part, "." | "..")
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            });
        if valid {
            Ok(Self(text.to_owned()))
        } else {
            Err(Error::blocked("Repository must be GitHub OWNER/REPO."))
        }
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
    pub(crate) fn owner(&self) -> &str {
        self.0.split('/').next().unwrap_or_default()
    }
}
impl<'de> Deserialize<'de> for Repository {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Model {
    Haiku,
    #[default]
    Sonnet,
}
impl Model {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Haiku => "haiku",
            Self::Sonnet => "sonnet",
        }
    }
}

fn base() -> String {
    "main".to_owned()
}
fn all_paths() -> Vec<String> {
    vec!["*".to_owned()]
}
fn timeout() -> u64 {
    900
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Check {
    pub name: String,
    pub argv: Vec<String>,
    #[serde(default = "all_paths")]
    pub paths: Vec<String>,
    #[serde(default = "timeout")]
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub repository: Repository,
    #[serde(default = "base")]
    pub base: String,
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub review_model: Model,
    #[serde(default)]
    pub hooks_path: Option<String>,
    pub checks: Vec<Check>,
    #[serde(default)]
    pub protected_paths: Vec<String>,
}
impl Config {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let config: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        config.validate()?;
        Ok(config)
    }
    pub(crate) fn validate(&self) -> Result<()> {
        if self.base.is_empty()
            || self.base.starts_with('-')
            || self.base.contains("..")
            || !self
                .base
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_./-".contains(&b))
        {
            return Err(Error::blocked("Invalid base branch."));
        }
        if self.checks.is_empty() {
            return Err(Error::blocked("Configure project checks before running."));
        }
        for check in &self.checks {
            if check.name.trim().is_empty()
                || check.argv.is_empty()
                || check.argv.iter().any(|s| s.is_empty())
                || check.paths.is_empty()
                || check.paths.iter().any(|s| s.is_empty() || s.len() > 1024)
                || !(1..=1800).contains(&check.timeout_seconds)
            {
                return Err(Error::blocked(
                    "A check needs a name, argv, paths, and a bounded timeout.",
                ));
            }
        }
        if let Some(hooks) = &self.hooks_path {
            let path = Path::new(hooks);
            if hooks.is_empty()
                || hooks.starts_with('-')
                || path.is_absolute()
                || path
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                return Err(Error::blocked("hooks_path must stay inside the project."));
            }
        }
        if self.protected_paths.iter().any(|p| p.is_empty()) {
            return Err(Error::blocked("Protected paths cannot be empty."));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Label {
    pub name: String,
}
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Actor {
    pub login: String,
    #[serde(rename = "type")]
    pub kind: String,
}
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Issue {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    /// `completed` or `not_planned` once closed.
    #[serde(default)]
    pub state_reason: Option<String>,
    pub labels: Vec<Label>,
    pub user: Actor,
}
impl Issue {
    pub(crate) fn has_label(&self, name: &str) -> bool {
        self.labels
            .iter()
            .any(|label| label.name.eq_ignore_ascii_case(name))
    }
    pub(crate) fn eligible(&self) -> bool {
        self.state == "open"
            && self.has_label("ai-ready")
            && !["ai-in-progress", "ai-blocked", "ai-review"]
                .iter()
                .any(|l| self.has_label(l))
    }
    pub(crate) fn same_scope(&self, other: &Self) -> bool {
        self.title == other.title && self.body == other.body
    }
    fn sections(&self) -> BTreeMap<&str, String> {
        let mut sections: BTreeMap<&str, String> = BTreeMap::new();
        let mut heading = None;
        for line in self.body.as_deref().unwrap_or_default().lines() {
            if let Some(name) = line
                .strip_prefix("### ")
                .or_else(|| line.strip_prefix("## "))
            {
                heading = Some(name.trim());
                sections.entry(name.trim()).or_default();
            } else if let Some(name) = heading {
                let content = sections.entry(name).or_default();
                content.push_str(line);
                content.push('\n');
            }
        }
        sections
    }
    /// Issue numbers of this repository from the optional `前提` section.
    /// A missing or empty section means no prerequisites.
    pub(crate) fn prerequisites(&self) -> Result<Vec<u64>> {
        let sections = self.sections();
        let text = sections.get("前提").map(|s| s.trim()).unwrap_or_default();
        if text.is_empty() || text == "_No response_" {
            return Ok(Vec::new());
        }
        let mut numbers = Vec::new();
        for token in text
            .split(|c: char| c == ',' || c == '、' || c.is_whitespace())
            .filter(|token| !token.is_empty())
        {
            let digits = token.strip_prefix('#').unwrap_or_default();
            let number = if digits.bytes().all(|b| b.is_ascii_digit()) {
                digits.parse::<u64>().ok().filter(|number| *number > 0)
            } else {
                None
            };
            let Some(number) = number else {
                let shown: String = token.chars().take(40).collect();
                return Err(Error::blocked(format!(
                    "前提 must list issues of this repository like `#47, #48`; cannot use `{shown}`."
                )));
            };
            if number == self.number {
                return Err(Error::blocked(format!(
                    "前提 lists the issue itself (#{number})."
                )));
            }
            if !numbers.contains(&number) {
                numbers.push(number);
            }
        }
        if numbers.len() > 20 {
            return Err(Error::blocked(
                "前提 lists too many issues; split the work.",
            ));
        }
        Ok(numbers)
    }
    pub(crate) fn task_kind(&self) -> Result<TaskKind> {
        let body = self.body.as_deref().unwrap_or_default();
        if body.len() > 24_000 || self.title.chars().count() > 300 {
            return Err(Error::blocked("Split the issue before approving it."));
        }
        let sections = self.sections();
        for name in ["目的", "対象範囲", "期待する動作", "完了条件", "検証方法"]
        {
            let text = sections.get(name).map(|v| v.trim()).unwrap_or_default();
            if text.is_empty() || text == "_No response_" {
                return Err(Error::blocked(format!("Missing issue section: {name}")));
            }
        }
        match sections.get("作業種別").map(|s| s.trim()) {
            Some("feature") => Ok(TaskKind::Feature),
            Some("bug") => Ok(TaskKind::Bug),
            Some("review") => Ok(TaskKind::Review),
            _ => Err(Error::blocked("作業種別 must be feature, bug, or review.")),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskKind {
    Feature,
    Bug,
    Review,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CheckStatus {
    Verified,
    Failed,
    NotRun,
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
}

pub(crate) fn matches(pattern: &str, text: &str) -> bool {
    let text: Vec<_> = text.chars().collect();
    let mut row = vec![false; text.len() + 1];
    row[0] = true;
    for ch in pattern.chars() {
        let mut next = vec![false; text.len() + 1];
        if ch == '*' {
            next[0] = row[0];
        }
        for index in 0..text.len() {
            next[index + 1] = if ch == '*' {
                row[index + 1] || next[index]
            } else {
                row[index] && (ch == '?' || ch == text[index])
            };
        }
        row = next;
    }
    row[text.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_rejects_shell_and_path_input() {
        assert!(Repository::parse("owner/project").is_ok());
        for text in ["../repo", "owner/repo;bad", "owner/repo/extra", ""] {
            assert!(Repository::parse(text).is_err(), "{text}");
        }
    }
    #[test]
    fn path_matching_supports_japanese_and_nested_files() {
        assert!(matches("src/**", "src/日本語/file.rs"));
        assert!(matches("*.sh", "scripts/check.sh"));
        assert!(!matches("src/**", "docs/src.md"));
    }
    fn with_body(body: &str) -> Issue {
        serde_json::from_value(serde_json::json!({
            "number":7,"title":"task","body":body,
            "state":"open","labels":[],"user":{"login":"owner","type":"User"}
        }))
        .expect("fixture")
    }
    #[test]
    fn prerequisites_are_optional_and_strictly_parsed() {
        let empty = [
            "### 目的\nx\n",
            "### 前提\n\n### 目的\nx\n",
            "### 前提\n_No response_\n",
        ];
        for body in empty {
            assert!(with_body(body).prerequisites().expect("none").is_empty());
        }
        assert_eq!(
            with_body("### 前提\n#12, #13 #12\n### 目的\nx\n")
                .prerequisites()
                .expect("list"),
            vec![12, 13]
        );
        let invalid = [
            "owner/other#3",
            "https://github.com/o/r/issues/3",
            "12",
            "#0",
            "#7",
            "see #3",
        ];
        for text in invalid {
            let issue = with_body(&format!("### 前提\n{text}\n"));
            assert!(issue.prerequisites().is_err(), "{text}");
        }
    }
    #[test]
    fn issue_requires_observable_acceptance_and_kind() {
        let mut issue: Issue = serde_json::from_value(serde_json::json!({
            "number":1,"title":"task","body":"### 作業種別\nfeature\n",
            "state":"open","labels":[{"name":"AI-READY"}],"user":{"login":"owner","type":"User"}
        }))
        .expect("fixture");
        assert!(issue.eligible());
        assert!(issue.task_kind().is_err());
        let body = issue.body.get_or_insert_default();
        for name in ["目的", "対象範囲", "期待する動作", "完了条件", "検証方法"]
        {
            body.push_str(&format!("\n## {name}\nObservable result.\n"));
        }
        assert_eq!(issue.task_kind().expect("complete"), TaskKind::Feature);
    }
}
