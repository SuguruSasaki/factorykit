# 0005 Rename Nikostack to FactoryKit

- Date: 2026-11-03
- Status: accepted

## Problem

The repository is no longer a personal stack for one product. It is the base
of an Issue-driven software factory that serves several product repositories
(see [0004](0004-factory-verification.md)). The name Nikostack also suggests a
relation to the Niko product, which is only one of its users.

## Decision

Rename the project to FactoryKit. Use `FactoryKit` for display and
`factorykit` for every identifier: the plugin and marketplace, the skill
directory `skills/factorykit/`, the Cargo package and CLI binary, the product
configuration `.factorykit/worker.json`, and the state root
`~/.local/state/factorykit`. Rename the GitHub repository to
`SuguruSasaki/factorykit`.

This is a clean rename. The CLI does not read `.nikostack/` or the old state
root. A fallback would keep two lock locations, so an old and a new worker
could claim the same issue.

Dated records keep the name they were written with, such as the
[2026-10-27 verification record](../verification/2026-10-27-rust-worker.md).
GitHub redirects their old links.

## Migration

1. Stop every running `nikostack worker`.
2. Rename the GitHub repository and update the local `origin` remote.
3. Install the new binary with `cargo install --path . --locked` and remove
   the old `nikostack` binary.
4. In each product, rename `.nikostack/` to `.factorykit/` and update its
   documentation through the product's normal change process.
5. Move `~/.local/state/nikostack` to `~/.local/state/factorykit` when no
   worker is running, then restart the workers.
6. Reinstall the plugin as `factorykit@factorykit`. The Claude Code command
   becomes `/factorykit:factorykit`.

## Verification

Run `./.agents/validate rust` and resolve local documentation links. After
migration, run `factorykit worker --project PATH --dry-run` for each product
before restarting `--watch`.
