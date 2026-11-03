---
name: factorykit
description: Route a product-development request through the project's instructions and evidence. Use for first-time project setup, a small feature, bug fix, or code review; load the small-feature procedure for a feature change. Do not use for unrelated writing or operations work.
disable-model-invocation: true
---

# factorykit

Use this as an entry point for work in the **target product repository**. Identify that repository before acting. Its instructions, existing skills, decisions, and verification recipe govern the work; this skill does not replace them. Keep product-specific commands and acceptance criteria there. Do not copy private data or credentials into factorykit.

For first-time adoption or a request to prepare the project's AI issue workflow,
read [references/setup-project.md](references/setup-project.md). Check the GitHub
repository and preview the shared issue-label setup, then add missing labels
within the user's authorized setup scope. Preserve existing labels. Ordinary
feature or review requests do not authorize changing repository settings.

1. Clarify the requested outcome and an observable completion condition. Read the relevant instructions, code, callers, tests, and past decisions before editing.
2. Classify the task as a feature, bug fix, or review. For a **small feature**, read [references/small-feature.md](references/small-feature.md) and follow it. For a bug fix, establish the observed failure and reproduce it before choosing the smallest correction. For a review, inspect and report findings without editing unless the user also asked for fixes.
3. Choose the smallest change that satisfies the outcome. Explain a design choice only when it affects behavior, compatibility, or verification; do not require multiple proposals for a simple change.
   Express important invariants through the target project's types, validation boundaries, and existing conventions where practical, so tools can reject invalid states and give the agent useful feedback. Avoid new abstractions or dependencies without a concrete need.
4. After editing, run the fastest relevant check, inspect the result, and correct failures before widening verification. Verify the relevant behavior with the project's own procedure. Distinguish tests, static checks, and real operation. Record the code version or working-tree diff, surface tested, action, expected result, observed result, and evidence location. Use `verified`, `failed`, `blocked`, or `not-run` for each check. Never label an unavailable check as verified.
   When behavior is captured in snapshots or expected output, review the diff before accepting an update. Make time, randomness, and external effects controllable when they cause unstable checks.
5. Report the change, design reason, observable behavior, evidence, and remaining gaps so a person can judge the result without reading the code. State what the checks confirmed and what they did not, and list the items a person should confirm on the product itself. If a problem repeats, decide whether its remedy belongs in this shared skill or in the product repository before adding a rule.

When resuming interrupted work, inspect the current files and prior evidence before repeating a change or check.

Work alone by default. Delegate only when a bounded independent task would help; specify its scope, model cost, concurrency, and stopping condition. Use an available low-cost model for routine delegated work. Do not silently substitute a more expensive model when the preferred one is unavailable.
