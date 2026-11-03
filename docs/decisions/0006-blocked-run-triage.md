# 0006 Triage blocked runs by cause

- Date: 2026-11-03
- Status: proposed

## Problem

A run that becomes `ai-blocked` waits for a person, whatever the cause. A
person must read the logs and decide what to do, even for a network error that
a retry would fix.

Retrying every blocked run automatically is not safe either. On
2026-11-03, Issue #16 was blocked because its acceptance criterion was wrong:
a grep for an absent label also matched a test that asserts the label is
absent. The read-only review suggested splitting the string so that the grep
would not find it. An automatic repair loop would have accepted that change
and published code that hid a specification error. This is the
"implementer accepts every suggestion" failure that
[0004](0004-factory-verification.md) describes.

## Decision

When a run is blocked, the controller classifies the cause. What happens
next depends on the class. The factory may repair implementation, but only a
person changes the specification or an approval.

### Classes and actions

| Class | Examples | Action |
| --- | --- | --- |
| Transient | GitHub API error, network failure, a timeout with no partial result | Retry the run once after a delay. A second failure is blocked. |
| Implementation | A finding backed by a failing check or a reproducible step | Repair within the run's existing repair limit. Do not raise the limit. |
| Specification | An acceptance criterion that contradicts the scope or the expected behavior, or one that cannot be met by correct code | Comment a proposed change to the issue text and stay blocked. A person edits the issue and applies `ai-ready` again. |
| Environment | A missing tool or credential, insufficient permission, or a change to a protected path | Comment what is missing and stay blocked. |

### Triage

- A fresh session reads the run's evidence: the issue, the diff, check logs,
  review output, and the receipt. It returns one class, the evidence for that
  class, and the proposed action. It has read-only file tools.
- The controller enforces the actions in the table. The triage session cannot
  edit the issue, apply labels, or start a repair.
- A finding without a failing check or a reproducible step is not
  implementation evidence. Classify it as specification when the criterion
  itself is the cause. Otherwise discard it.
- A change that satisfies a criterion literally while avoiding its intent
  is never a repair. Examples are splitting a string to avoid a grep and
  skipping a test.
- If triage fails or is unsure, the run stays blocked as it does today.

### Learning from blocked runs

The controller records each run's class and cause in its private evidence.
When the same cause repeats, triage proposes a rule for the place that would
have prevented it. That place is the issue checklist, the product's invariant
ledger, or this shared skill. A person accepts the rule through an ordinary
change. Rules are not added automatically.

## Tradeoffs

- Triage is another AI call for each blocked run. A blocked run is the
  exception, so the cost is small compared with the implementation.
- A wrong classification can still cause one extra retry or repair. The
  existing limits bound it, and specification and approval stay with a
  person.
- A person still edits specifications. This keeps 0004's division of roles:
  people write the specification, and the factory makes changes verifiable.

## Rollout

Deliver this with 0004 step 2 (the verifier). Triage uses the same
evidence rules as the verifier and replaces the comment that today only says
to inspect the private log. Add contract tests with fake GitHub and AI for
each class. One of them reproduces Issue #16: a criterion that also matches
a negative test must be classified as specification, and no repair is
started.

## Verification

Contract tests cover each class, the retry and repair limits, and the
rule that triage cannot change the issue or its labels. A live trial records
the class and outcome of the blocked runs in its first week of operation.
