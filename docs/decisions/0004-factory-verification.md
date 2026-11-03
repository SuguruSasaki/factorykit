# 0004 Factory verification replaces human code review

- Date: 2026-11-03
- Status: proposed

## Problem

FactoryKit aims to be a factory that produces products. People decide what to
build and verify behavior when needed; they no longer review code. Version 0.2
still assumes human code review. The entry skill says passing checks do not
replace human review, `ai-review` waits for a human reviewer, and the worker's
only gate beyond configured checks is a read-only AI review. That review is an
opinion, not evidence, and implementers tend to accept every suggestion.

The worker is also serial, and every merge waits for a person. Neither scales to
a queue of approved issues.

## Decision

FactoryKit, not a human reviewer, makes a change verifiable. A person writes the
specification and verifies behavior that the factory cannot check.

### Roles

- A person writes each issue: purpose, scope, acceptance criteria, and
  verification method. The person owns the product's invariant ledger and
  applies `ai-ready`. The person never reviews code.
- One controller claims approved issues. Each claimed issue gets its own clone,
  implementer, and verifier: one agent per issue, never several issues per agent.
- One merge manager integrates verified pull requests in order.

### Evidence instead of opinion

- Map each acceptance criterion to an executed check, or mark it as a human
  behavior check. A criterion without either blocks publication.
- Record red-to-green evidence: the controller runs new or changed tests on the
  base commit, where they must fail, and on the head commit, where they must pass.
- Run the checks listed in the product's invariant ledger on every change. For a
  touched invariant, the controller applies the ledger's recorded mutation and
  confirms that the check fails.
- Replace the read-only review with a verifier. In a fresh session, it reads the
  issue, the exact diff, and the check logs. It reports `pass`, `fail`, or
  `needs-human` for each criterion. A finding without a failing check or a
  reproducible step is discarded.
- The implementer still edits files only. The controller owns every command, so
  the implementer cannot run, skip, or edit the checks.

### Parallel work

The controller claims up to N `ai-ready` issues one at a time, so two workers
never claim the same issue. N defaults to 2. Claimed issues then run in
parallel. A blocked issue does not stop the others.

### Merge manager

The merge manager handles pull requests oldest first, one at a time.

1. If `main` has moved and merges without conflict, it merges `main` into the
   branch and reruns the checks on the new head.
2. If every check passes and the pull request has no `human-verify` label, it
   merges the pull request.
3. If the pull request has `human-verify`, it leaves the merge to a person.
4. If the branch conflicts with `main`, it never edits code. It returns the
   issue to `ai-ready` to be implemented again from the latest `main`. A second
   conflict on the same issue becomes `ai-blocked`.

Installing or updating a product stays a separate, explicitly approved step.

### Labels

Add `human-verify`: factory evidence passed, but a person must verify behavior
and merge. The issue author applies it with `ai-ready` when a criterion needs a
person, such as a Slack conversation or a Mac UI. The verifier applies it when it
reports `needs-human`. `ai-review` now means that a pull request awaits the merge
manager.

## Tradeoffs

- Quality rests on the person-written specification. A wrong criterion yields
  wrong code that passes. The invariant ledger should grow from each fix so the
  same class of mistake is not repeated.
- No reviewer looks for problems that nobody specified. Incidents, not opinions,
  add new invariants.
- Merging without a person reaches `main` sooner. A bad merge is reverted;
  installation still needs approval.
- Parallel work multiplies local build load and subscription usage, and makes
  conflicts more likely. Start with N = 2 and measure before raising it.

## Rollout

Deliver each step as its own issue, verification before autonomy, so automatic
merge never ships before the verifier:

1. Remove human code review assumptions from the skill, label descriptions,
   and maintainer instructions.
2. Add the verifier, criterion mapping, and red-to-green evidence, replacing
   the read-only review.
3. Add the product-owned invariant ledger and mutation evidence.
4. Make the controller claim and run up to N issues in parallel.
5. Add the merge manager and the `human-verify` label.

## Verification

Each step adds contract tests with fake GitHub and AI, as the worker does
today. Before enabling automatic merge on a product, run a live trial that
records build time, subscription usage, and conflicts for N = 2. This decision
replaces the serial-worker and no-automatic-merge parts of
[0003](0003-local-issue-worker.md).
