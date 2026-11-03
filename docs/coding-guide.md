# Coding guide adoption

FactoryKit uses the Rust and Markdown skills from
[SuguruSasaki/ai-agent-coding-guide](https://github.com/SuguruSasaki/ai-agent-coding-guide),
installed from revision `0f7630e22480140fd49111300342bd0e26601245`.
These are development rules for this repository; plugin installation does
not overwrite a product repository's own rules.

## Shared rules for both agents

The guide's installer added the rules to `.agents/skills/rust-coding/` and
`.agents/skills/markdown-docs/`, plus the dispatch command `.agents/validate`.
The root [AGENTS.md](../AGENTS.md) selects applicable rules; Claude Code's
[CLAUDE.md](../CLAUDE.md) imports it. Relative links in `.claude/skills/`
point to the same skills. Edit or update the source once, rather than
maintaining separate copies for each agent.

The existing FactoryKit entry skill remains at `skills/factorykit/`.
The imported guide is not a new runtime dependency.

## Validate a change

```sh
./.agents/validate rust
git diff --check
```

The Rust dispatcher runs format checking, Clippy with warnings denied, and
the Cargo workspace tests. Review ownership, errors, state transitions and
observable behavior as well: automation cannot decide every design issue.
For Markdown, check local links, heading structure, whitespace, commands and
rendered readability as described by the Markdown skill.

## Update the imported guide

Review a chosen upstream revision in a separate checkout and run its installer
from this repository root:

```sh
bash /path/to/ai-agent-coding-guide/scripts/install.sh install rust markdown
```

Inspect the diff, keep the Claude skill links pointed at the shared source,
rerun validation, and update the revision recorded above. The installer is
designed to preserve unrelated skills and existing project instructions.
