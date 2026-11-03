# AI task checklist

Use this checklist before asking an AI agent to work on an issue. A clear issue
lets a reviewer confirm the intent and lets the agent stop at the right boundary.

## Labels

- `ai-ready`: a human has approved work on the scope described in the issue.
  Do not start work without this label.
- `ai-review`: the agent has reported back or opened a Draft PR, and a human
  must check the result before anything else happens.

## Before starting

Prepare these five items in the issue body.

### Purpose

- State why the change is needed, in one or two sentences.
- Name the user or maintainer who benefits.

### Scope

- List the files, directories, or surfaces that may change.
- List what must not change (for example, unrelated code, config, or docs).
- Name protected paths the agent must not edit.

### Expected behavior

- Describe what should be observable after the change, from the user's view.
- Give one concrete example input and output where practical.

### Acceptance criteria

No person reviews the code, so each criterion must be decidable from evidence.
Write each item as an executable check:

- Give the input or action, the command or test that observes it, and the
  expected result.
- Prefer a test the agent adds and the controller runs. It should fail before
  the change and pass after it.
- Mark an item that a machine cannot decide as `人が確認` and say what the person
  observes, such as a Slack reply or a Mac UI. Keep these items few.
- Keep the list short. Each item should be independently testable.

Avoid criteria that only say something exists or is described. A verifier can
only judge them by reading, which is an opinion, not evidence. The commands in
this table are illustrative.

| Hard to verify | Executable |
| --- | --- |
| The design describes the many-to-many links. | `cargo test knowledge::links` passes, including a test where one claim serves two purposes. |
| Search handles stale articles. | Given fixture article A dated more than 365 days ago, `niko search --json "ssh"` returns A with `"stale": true`. |

A design or specification issue produces the rules that later issues follow.
Mark its judgment items as `人が確認`, and also state the structural checks a
machine can run, such as required headings or links that must resolve.

### Verification method

- Name the commands or steps the controller will run, such as `grep`,
  `git diff --check`, or the project's test suite. Map each acceptance
  criterion to one of them or to `人が確認`.
- State what is out of scope for verification, such as live product or
  external service behavior.

## Before marking `ai-ready`

- [ ] Purpose, Scope, Expected behavior, Acceptance criteria, and
      Verification method are all filled in.
- [ ] The scope is small enough to review in one Draft PR.
- [ ] Every acceptance criterion is an executable check or is marked `人が確認`.
- [ ] The issue is a single task, not a parent that only lists child issues.
      Do not mark a parent issue `ai-ready`.
- [ ] Every prerequisite issue is already merged. The worker does not read
      prerequisites yet, so mark dependent issues `ai-ready` one at a time.
- [ ] Protected paths and secrets are not part of the requested change.
- [ ] Someone with authority has approved the scope.

## After the agent reports

- Read the report and the Draft PR diff; do not rely on passing checks alone.
- Confirm each acceptance criterion against the evidence.
- Record any check that was not run or was blocked.

## Human decisions

Merging a PR and updating an installed app are separate decisions. Both need
human review. Automated merge and automated installation are not performed by
this workflow.
