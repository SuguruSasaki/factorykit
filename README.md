# factorykit

FactoryKit provides a shared development workflow for coding agents across product repositories. Version 0.2 adds project label setup and an optional serial Mac Issue worker to the entry skill and small-feature procedure. Each product project owns its operations, expected behavior, and verification methods.

## Install as a plugin

This repository is the `factorykit` plugin. `skills/factorykit/` is its single skill source. The root `plugin.json` packages it for Codex, and `.claude-plugin/plugin.json` provides Claude Code compatibility. The package contains no MCP server or product-specific settings.

For Codex CLI, add this repository as a marketplace and install the plugin:

```text
codex plugin marketplace add SuguruSasaki/factorykit
codex plugin add factorykit@factorykit
```

For Claude Code:

```text
claude plugin marketplace add SuguruSasaki/factorykit
claude plugin install factorykit@factorykit
```

## Activate it in a product repository

On first adoption, follow [project setup](skills/factorykit/references/setup-project.md).
The shared helper previews and creates missing AI issue labels without replacing
existing labels or starting work. Plugin installation itself does not write to GitHub.

For optional Issue-to-draft-PR automation on your Mac, follow the
[local worker guide](docs/issue-worker.md). The worker uses the same skill source
and product-owned checks; it does not require a global plugin installation.

Label preparation and the worker use one Rust CLI. From a source checkout,
with Rust 1.89 or newer and GitHub CLI installed:

```sh
cargo install --path . --locked
factorykit setup --repository OWNER/REPO
factorykit setup --repository OWNER/REPO --apply
```

The preview is read-only on GitHub; `--apply` creates missing labels.
Installing the plugin distributes the skill; install the optional CLI
separately. The runtime does not require Python.

Install the plugin once with the commands above. Then start the agent from the product repository you want to change:

```sh
cd /path/to/product-repository
claude  # or: codex
```

At the start of a Claude Code message, invoke `/factorykit:factorykit` followed by a concrete feature, bug fix, or review request. [Claude Code qualifies plugin skills with the plugin name](https://code.claude.com/docs/en/plugins), so this is the plugin command corresponding to a direct command such as [pstack's `/poteto-mode`](https://github.com/cursor/plugins/blob/main/pstack/README.md):

```text
/factorykit:factorykit Fix the failing login test and verify the affected behavior.
```

In Codex, open `/skills` and select `factorykit`, or start the request with `$factorykit`:

```text
$factorykit Fix the failing login test and verify the affected behavior.
```

Invoke the skill when you want its workflow for the current request. It does not toggle a persistent mode or replace the agent's system message. The skill is configured for manual invocation; it does not require a standing instruction in each product repository. The product's own instructions, specifications, and verification commands still govern the work. Check that the agent reports the code state, checks it ran, observed results, and any unverified areas.

See the [OpenAI Docs packaging guide](https://developers.openai.com/plugins/build/plugins) and the [Claude Code marketplace guide](https://code.claude.com/docs/en/plugin-marketplaces) for installation details.

## Use the skill in this repository

When you open this directory in Codex or Claude Code, you can invoke the entry skill as `$factorykit` or `/factorykit`, respectively. After installing the plugin in another repository, Claude Code uses the qualified name `/factorykit:factorykit`. The entry point is [SKILL.md](skills/factorykit/SKILL.md); the [small feature procedure](skills/factorykit/references/small-feature.md) provides more detail.

`.agents/skills/factorykit` and `.claude/skills/factorykit` are relative symbolic links to the one source at `skills/factorykit/`. Edit the source only. Keep product-specific settings in the product repository.

## Verification status

- Version 0.2 replaces the label helper and local Issue worker with Rust.
  See [worker verification](docs/issue-worker.md#verification-and-remaining-limits)
  for test coverage and the distinction between fixtures and live behavior.
- The prior helper prepared Niko's labels without replacing existing labels.
  Rust runtime verification is recorded separately; it does not establish
  that Niko's external integrations or installed application work.
- Rust label setup was applied to FactoryKit and rerun without changes.
  After two stopped trials, a live Haiku run completed Issue-to-Draft-PR
  processing for a small documentation task, creating [PR #5](https://github.com/SuguruSasaki/factorykit/pull/5).
  [the trial record](docs/verification/2026-10-27-rust-worker.md) describes
  fixes, evidence, and the remaining product-specific verification.

- The skill metadata, relative links, and local discovery paths were checked. The Claude Code manifest and marketplace passed validation.
- Codex CLI 0.164.0 and Claude Code 2.1.310 installed version 0.1.2 from the local marketplace and discovered the skill. Version 0.1.0 was also installed from GitHub in both CLIs. Temporary installations and registrations were removed.
- Invoking the current version in a product session and applying it to a real product change remain untested.

The [initial design decision](docs/decisions/0001-entry-skill-and-small-feature.md) explains the entry skill and scope. The [plugin distribution decision](docs/decisions/0002-plugin-distribution.md) records the packaging choice. The [reliable software engineering reference](docs/references/jane-street-reliable-software.md) covers shared principles and gaps; it does not replace product-specific rules or verification procedures.

## Developing FactoryKit

This repository adopts [ai-agent-coding-guide](https://github.com/SuguruSasaki/ai-agent-coding-guide)
for Rust and Markdown work. Codex and Claude Code read the same installed
rules. See [coding guide adoption](docs/coding-guide.md) for the source
revision and update procedure. Run `./.agents/validate rust` before submitting
runtime changes.
