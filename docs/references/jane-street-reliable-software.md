# Principles for Reliable Software Engineering

## Source and scope

- Talk: Ron Minsky (Jane Street), “[Now more than ever: building reliable software in the age of agents](https://www.youtube.com/watch?v=rUYP4C29yCw),” Bug Bash 2026.
- Sources checked: the [organizer's description of the talk](https://antithesis.com/bugbash/talks/now-more-than-ever/), [Jane Street's explanation of expect tests](https://blog.janestreet.com/testing-with-expectations/), and the [official Verus guide](https://verus-lang.github.io/verus/guide/overview.html).
- Verification limit: The video's spoken content and subtitles were not checked directly. References to the talk's themes below are limited to what the organizer's description supports. The operational examples, priorities, and FactoryKit decisions in this document are our own proposals, not attributed statements by the speaker.

The organizer describes type systems as a way to enforce invariants and give agents fast feedback; expect tests as a way to expose behavior and support deterministic simulation at the library level; code review as a way to maintain quality and human understanding; and formal verification as a growing area of interest. The organizer also describes the goal as maximizing human effectiveness and understanding rather than replacing people. FactoryKit applies these themes to a workflow in which agents verify their changes and people can assess the evidence. It does not prescribe the same tools for every project.

## Adopted principles and examples

### 1. Make invalid states difficult to create

Identify the invariants of inputs and state. Enforce them with the target language's types, constructors, boundary validation, and static analysis where appropriate. A well-chosen type is an executable instruction to the agent: it states which values and transitions are allowed, and the compiler gives feedback when generated code violates that contract. Do not rely on a prompt as the only constraint. Type checking alone cannot guarantee that runtime input or business behavior is correct.

Example: Make the transition from `unvalidated input -> validated value -> operation` explicit. In Rust, consider enums or types with private fields and validating constructors. In TypeScript, consider discriminated unions together with runtime input validation. Each target repository chooses its own language-specific conventions.

### 2. Keep verification loops short

Make relevant compilation or type checks, lint, and tests executable by the agent. After a change, run the fastest relevant check promptly, inspect its result, and use that feedback to correct the implementation before widening the check. Start with checks that narrow the cause of failure, then expand according to the affected behavior. On failure, retain a reproducible action and output that identifies the failing target, expected result, and observed result. Tie results to the exact code state and distinguish unavailable checks from passing checks.

Example: Run the affected module's type check and focused tests first. Add integration tests and checks through the actual user-facing surface when needed. The target project owns specific commands and ordering.

### 3. Test observable behavior

Test meaningful inputs, outputs, state transitions, and side effects. Expect tests and snapshot tests can make behavior changes visible to agents and reviewers. Compare every proposed snapshot update with the intended behavior; never approve updates unconditionally. Prefer small scenarios that make failures easy to diagnose over broad snapshots of entire views. Avoid tests that only repeat the implementation.

Example: Fix a normal and an error input for an API, then check its response and persisted side effects. Show the output diff in a form a reviewer can compare with the intended change.

### 4. Reproduce test conditions

When nondeterminism obstructs verification, make time, randomness, network behavior, and external services controllable so an agent can rerun the same case. Fix or mock nondeterministic LLM responses where needed. A fixed response does not verify the real service connection or operation; those may need separate checks.

Example: Inject a clock and random seed, reproduce an external API failure, and check retry behavior and side effects. Record the conditions needed to reproduce the failure.

### 5. Keep changes reviewable

Keep diffs small and grouped by purpose. Explain the design choice, intended behavior change, impact, surfaces checked, and what remains unverified so a person can judge the change. A passing test supports only the behavior that test examined; it does not establish the soundness of the entire design or user experience. Human review still needs to consider unintended behavior, compatibility, and readability, especially when agents produce code quickly.

### 6. Control complexity

Read the existing design and conventions. Avoid duplicate responsibilities and abstractions without a concrete need. Add a dependency only when its value over existing options and its maintenance cost can be explained. Consider whether the next change will be easy, not only how quickly this one can be written. Reasoned exceptions are allowed.

## Future exploration: formal verification

When important properties cannot be adequately established by types and ordinary tests, evaluate specification and proof for a bounded part of the system. [Verus](https://verus-lang.github.io/verus/guide/overview.html) is one option for statically verifying Rust code against stated specifications. Agents may reduce the cost of drafting specifications and proofs, but that benefit must be demonstrated on a real case; an agent-written proof is not evidence until the verifier checks it. This does not require adopting Rust or formal methods across every project. Identify the property to prove, the verification boundary, unsupported parts of the toolchain, and the cost of maintaining the proof. A successful proof still requires human review of the specification itself.

## Goal: increase human effectiveness and understanding

Use automation to give people stronger evidence and more time to assess design and behavior. Do not optimize for code volume or for eliminating human reading. A proposed workflow improvement should help an agent find errors sooner, help a reviewer understand what changed, or make an important property easier to verify. If it adds ceremony without one of those benefits, keep it out of the shared procedure.

## Baseline assessment and improvement candidates

This assessment covers `AGENTS.md`, `README.md`, `skills/nikostack/`, and `docs/decisions/0001-entry-skill-and-small-feature.md` before the reliable software principles were added. Newly stated guidance is distinct from implementation and verification in a target product. This assessment does not evaluate any product repository.

The initial design decision records Niko's `verify-niko` as an example used to check the shared-versus-product boundary and assigns language-specific guidance to `ai-agent-coding-guide`. No statement in this repository identifies a principle as originating from pstack, so this document does not infer that provenance.

| Principle | Baseline | Evidence and gap |
| --- | --- | --- |
| Prevent invalid states | Not Supported | The entry procedure required reading target conventions and code but did not state how to enforce invariants through types or input boundaries. This principle was subsequently added to the entry skill. |
| Short verification loops | Partially Supported | The entry and small feature procedures required static checks, tests, and status-specific evidence. There was no shared automation environment or command. |
| Behavior-oriented testing | Partially Supported | The existing procedure covered observable completion conditions, meaningful regression tests, and checks through the user-facing surface. Expect and snapshot tests remained product choices. |
| Deterministic simulation | Not Supported | The procedure did not address control of time, randomness, or external effects. Conditional guidance was subsequently added, but there is no implementation example yet. |
| Reviewability | Already Supported | Existing procedures required small diffs, design reasons, evidence tied to code state, and disclosure of unverified areas. |
| Human effectiveness and understanding | Partially Supported | Existing review evidence helps people assess changes, but the goal of increasing human understanding had not been stated explicitly. It is now part of the shared guidance. |
| Complexity control | Partially Supported | The procedure required reading the existing design and making the smallest change. Guidance on duplication and dependencies was added later; it has not yet been tested in a product case. |
| Formal verification | Future Exploration | Consider it only for important, bounded properties. There is no shared procedure or adoption case yet. |

| Improvement candidate | Priority | Expected effect | Cost and condition |
| --- | --- | --- | --- |
| Record failure output and reproduction steps in a real product change, then identify friction in the verification loop | High | Help agents isolate failures and report unverified work accurately | Low; do this in the next application case. |
| Establish a review step for snapshot diffs in a product that uses snapshots | Medium | Make unintended behavior changes easier to detect | Low to medium; wait for a relevant product. |
| Add injectable boundaries in a product affected by unstable time, randomness, or external effects | Medium | Make failures reproducible and verification results comparable | Medium; first establish the actual source of instability. |
| Trial formal verification for a high-value invariant | Low | Establish properties ordinary tests may miss | High; first define a specific target and property. |

This repository contains only shared procedures. Language-specific rules, product specifications, commands, and test infrastructure belong in target projects. Consider a new shared rule when the same issue recurs across multiple applications.
