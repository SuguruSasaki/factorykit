# 0002 Plugin distribution

- Date: 2026-10-26
- Status: Adopted (verified with local and GitHub marketplaces)

## Problem

The original relative symbolic links exposed the skill to Codex and Claude Code inside this repository. Using it in another product repository required the user to place the skill manually.

## Decision

Treat the repository root as the plugin package and keep `skills/factorykit/` as the single skill source. The root `plugin.json` is the portable manifest for Codex; `.claude-plugin/plugin.json` is the Claude Code compatibility manifest. Both marketplaces point to the repository root. Neither duplicates the skill content.

Users install the plugin explicitly. It provides only the shared workflow. Product-specific specifications, verification commands, credentials, and MCP servers stay out of the package. The target product's instructions remain authoritative for its work.

Users invoke the skill on demand from the target repository. Claude Code exposes the plugin skill as `/factorykit:factorykit`; Codex exposes it through `/skills` or `$factorykit`. Host-specific metadata disables implicit invocation where supported. Invocation does not replace the host's system message or create a persistent mode.

## Verification and remaining work

Codex CLI 0.164.0 and Claude Code 2.1.310 installed version 0.1.1 from the local marketplace, including the manual-invocation metadata. Version 0.1.0 was installed from GitHub in both CLIs. The temporary installations and marketplace registrations were removed.

Invoking version 0.1.1 in a product session and applying it to a real product change remain untested. Submission to a public plugin directory would require meeting its separate publication requirements, including a license.
