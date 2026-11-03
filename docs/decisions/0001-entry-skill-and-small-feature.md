# 0001 Entry skill and small feature changes

- Date: 2026-10-26
- Status: Adopted (initial stage)

## Problem

Recreating an agent's development procedure in each product weakens the link between completion conditions and verification evidence. Putting product-specific verification commands in a shared procedure would cause that procedure to drift as the number of products grows.

## Decision and rationale

Keep `skills/factorykit/` as the only Agent Skills source, with relative links from the Codex and Claude Code discovery locations. The entry skill routes feature, bug fix, and review requests. At this stage, provide a detailed procedure only for small feature changes. The entry skill requires agents to read the target project first, distinguish verification states, and consider delegation cost.

Keep each product's Feature Map and operation commands in its own repository. Niko's `verify-niko` was consulted to confirm this boundary. Use the existing `ai-agent-coding-guide` for language-specific guidance rather than copying it here.

Relative links expose the shared source to both local tools without maintaining duplicate files. At the time of this decision, a separate installation script or plugin would have been premature because the needs of a second target project were still unknown. [Decision 0002](0002-plugin-distribution.md) later adopted plugin packaging.

## Verification and next evidence

As of 2026-10-26, Ruby parsed the YAML frontmatter; both relative links resolved to the source; and Markdown relative references were checked. Codex CLI 0.164.0 and Claude Code 2.1.310 were available locally. The official `quick_validate.py` could not run because PyYAML was unavailable, so its result was not counted as a pass. Invocation from both tools and application to a product feature change had not yet been tested at this stage.

The next product feature change should inform the installation method and any skill for setting up verification. If product-specific exceptions recur across target projects, revisit the boundary between shared and product-owned procedures.
