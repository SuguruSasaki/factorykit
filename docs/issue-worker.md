# Local Issue worker

The Mac worker turns explicitly approved GitHub issues into an isolated
implementation, deterministic checks, a fresh read-only AI review, and a
draft PR. It uses the local Claude subscription login. It never merges,
updates the installed product, or installs a daemon.

## Requirements and trust

Use Rust 1.89 or newer on macOS, Git, authenticated GitHub CLI, and a Claude
Code version supporting --restricted, --permission-prompts, and --json-schema.
The CLI flags were checked against the available local CLI; run the smoke
check before depending on another version. No API-key fallback is configured.

Run only on a private, trusted workstation for repositories you control.
Restricted file tools and absent shell/MCP tools constrain Claude, but project
builds and tests still execute trusted code as your Mac user. This is not an
OS sandbox for hostile repositories. The worker checks that both the issue
author and latest ai-ready approver are human collaborators with write,
maintain, or admin permission.

## Prepare

First follow [project setup](../skills/nikostack/references/setup-project.md).
The project owns .nikostack/worker.json. Set repository, base, checks, any
extra protected_paths, and optional hooks_path. Each check is a name and argv
array; optional paths selects affected files and timeout_seconds bounds the
command. Do not put credentials or personal data in the configuration.

Install once from the nikostack source checkout, then replace the product path:

```sh
cargo install --path . --locked
nikostack worker --project /path/to/product --dry-run
nikostack worker --project /path/to/product
nikostack worker --project /path/to/product --watch
```

Default mode handles at most one task. --watch polls every 120 seconds and
drains eligible work serially. An empty queue invokes no AI. --dry-run reads
the queue without claiming issues or checking Claude authentication.
--state-dir changes the private local state root. Do not run a second worker
on another machine or with a different state root for the same repository:
the file lock coordinates one Mac and one state root, not a distributed fleet.

The runtime and label setup are Rust; Python is not required. Rebuild the
binary after changing the shared skill: its workflow and label definitions
are embedded at compile time. The plugin and CLI have separate installation
steps. Without installing globally, use `cargo run --locked -- worker ...`.

## Workflow and authority

1. A person writes a task using the product's issue form and applies ai-ready.
2. The worker verifies the approver, skips a task with an existing draft PR,
   adds ai-in-progress, removes ai-ready, and records a run and branch.
3. It clones to a private run directory and starts from the remote base.
   The original development checkout is not changed.
4. Claude reads the shared nikostack source plus product instructions and
   edits through file tools. Shell, GitHub, MCP, and subagent tools are absent.
   The controller runs the configured checks. One check-failure repair is
   allowed; repeated failure blocks the issue. The repair receives at most
   10,000 characters of the failed check's log: failure summary lines
   (`Validation results:`, `test result: FAILED`), lines around `FAILED`,
   `panicked at`, `error:`, `Diff in` and `warning:`, then the ends of stdout
   and stderr. Omitted line ranges are marked, and the full log's line count
   is stated. A log with no failure markers is passed as its last 10,000
   characters.
5. A fresh Claude session reviews the issue and exact staged diff with
   read-only file tools. Failed or unavailable review stops publication.
6. Before publication, the controller checks that the issue is still open,
   its scope has not changed, and it still has ai-in-progress. It commits
   with the authenticated GitHub user's public noreply identity and pushes
   only the dedicated ai/issue-N branch. Configured push hooks remain active.
7. It opens a draft PR with Closes #N, the head SHA, check statuses, and
   remaining live checks, then moves the issue to ai-review.
8. It adds the PR label `human-merge`, then switches the PR to Ready for
   review. The label means the factory's work and checks are finished and a
   person merges after confirming the listed remaining checks. It is not
   permission to update the application. If the label or the switch fails,
   the PR stays Draft and the reason is commented on the issue, which is
   still moved to ai-review. Merging stays with a person. `nikostack setup`
   adds the `human-merge` label without changing existing labels.

A review-only task uses read-only tools and reports findings without a PR.
The worker reports code-review feedback as an issue comment under the current
GitHub identity; it is not an independent human approval or GitHub review gate.

Both AI roles default to sonnet, with medium effort for implementation and
low for review. Implementation is capped at 24 turns and 20 minutes; review
at 12 turns and 10 minutes. One repair uses a fresh bounded call. These caps
do not constitute a fixed credit or dollar budget. The model is explicit,
and permits `haiku` or `sonnet` in each model field. Choose Haiku for small,
bounded documentation changes. The controller records the actual model,
turn count, duration, and reported API-equivalent cost in private usage JSON.
That cost is not a subscription billing statement. A different model family
in Claude's usage report stops further calls; the controller cannot prevent
an internal CLI fallback that has already happened. Prepare complex
architecture decisions before applying ai-ready.

Claude children receive the documented nonessential-traffic and telemetry
opt-outs; parent settings stay unchanged. API-key and alternate-provider
environment selectors are removed from those children. These controls follow
[Claude network configuration](https://code.claude.com/docs/en/network-config)
and do not replace review of the machine's network policy.

All controller children also receive `GH_TELEMETRY=false` and
`DO_NOT_TRACK=1`, including nested Git credential helpers and checks.
This uses [GitHub CLI's documented opt-out](https://docs.github.com/en/github-cli/github-cli/github-cli-telemetry)
without changing the user's global configuration or redirecting traffic.
It controls the standard GitHub CLI collector, not arbitrary extensions.

## Prerequisites between issues

An issue may have an optional 前提 section listing issues of the same
repository that must be done first, written as `#47, #48` (commas or spaces).
No section, or an empty one, means no prerequisite. Do not put ai-ready on a
parent issue that only tracks children; label the child issues, each listing
its predecessors in 前提.

Before picking an approved ai-ready issue, the worker checks every listed issue:

- All closed as completed: the issue is picked as usual.
- Any still open: the issue is skipped with its labels unchanged and no comment.
  The next patrol checks again. The skip and the waiting numbers are printed
  to stderr and returned in the result (`status: waiting` or a `waiting` list).
- Closed for any other reason (not planned, duplicate, unknown): the issue
  becomes ai-blocked with an explanatory comment.
- A nonexistent number, another repository or a malformed entry, the issue
  itself, or a cycle among open prerequisites: ai-blocked with a comment.

After fixing the cause, remove ai-blocked and add ai-ready again.
--dry-run lists `ready`, `waiting` (with `waiting_for`), and `blocked`
issues separately without writing. The worker relies on the closed-as-completed
state; it does not verify that a merge reached the base branch.

## Stop and recover

Ctrl-C or SIGTERM stops the worker and its current child process group.
An interrupted task can remain ai-in-progress. Inspect its receipt, retained
checkout, remote branch, and PR before deciding whether to finish manually
or requeue. There is no automatic recovery that repeats an interrupted AI run.

Failures become ai-blocked with a reason. If GitHub cannot record that state,
the polling worker exits to avoid retrying the same task indefinitely.
Resolve the cause, inspect any retained remote branch, remove ai-blocked,
and explicitly add ai-ready. Existing PRs go back to ai-review instead of
creating a second PR. A pre-existing branch without a PR may require manual
recovery; force-push is not used.

Private state is under ~/.local/state/nikostack/owner__repo unless overridden
(repository names are lowercased for locking).
Each run retains its issue snapshot, base SHA, check logs for both attempts,
AI outputs, and receipt. Permissions are private on creation; do not share
the directory or commit it. Logs can contain project information.

## Verification and remaining limits

Run the contract tests:

```sh
./.agents/validate rust
```

They use real local Git repositories and fake GitHub/AI, covering successful
publication, no-op queues, duplicate PRs, failed checks, protected instructions,
protected-file renames, review mutation, scope drift, timeout, locks, and
idempotent label setup. Real Claude tool access and real GitHub interactions
need separate smoke checks; tests alone do not establish those connections.

The [Rust trial record](verification/2026-10-27-rust-worker.md) covers real
label setup, stopping on failures, and a completed documentation task from
Issue to Draft PR. The revised repair-feedback path has contract coverage
but was not needed in the successful live trial. This does not establish
unattended operation or verify a product's behavior.

Product behavior verification remains the product's responsibility. A draft
PR and passing code review do not establish that Slack, a Mac UI, external
writes, or actual user acceptance worked. Record unavailable checks explicitly.
The worker leaves final review, merge, and installed-product updates to the
operator. Unattended merge needs a separate head-bound approval and required
check design; it is not enabled here.
