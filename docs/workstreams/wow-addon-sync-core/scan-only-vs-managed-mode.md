# Scan-Only vs Managed Mode

## Status

Active product note for `wow-addon-sync-core` as of 2026-04-22.

## Purpose

HearthSync needs one explicit product explanation for a boundary that is easy to blur during CLI
and future desktop work:

- the tool should recognize and inspect WoW installs without first mutating the client tree
- tracked addon workflows still need persistent state for identity, policy, reproducibility, and
  adopted local snapshots

This note names those two operating postures so future UX, runtime, and storage decisions can stay
coherent.

## Definitions

### Scan-Only

Scan-only means HearthSync is discovering, inspecting, validating, or previewing WoW content
without creating or mutating managed addon state for that installation.

Typical scan-only examples:

- installation discovery and inspection
- health checks
- bundle inspection
- config-package or external-package inspection
- addon catalog search

The important rule is not “reads no files”; the rule is “does not require HearthSync to establish
tracked addon truth for this installation”.

### Managed Mode

Managed mode means the product is operating on tracked addon identity and therefore depends on
persisted HearthSync-owned state.

This includes flows that:

- install, update, remove, adopt, relink, or attach tracked addons
- write or consume addon lock state
- write or consume mutable addon policy
- snapshot local addon directories into adopted archives

Managed mode is broader than “write right now”.
Some managed-mode entrypoints only read or verify state in a given invocation, but their meaning
still depends on the persisted tracked-state contract.

## Why Managed State Still Exists

Tracked addon workflows need durable truth that cannot be reconstructed safely from the live
`Interface/AddOns` directory alone.

Specifically, HearthSync needs persistent state for:

- tracked package identity and source attribution
- reproducible addon lock snapshots
- mutable update policy such as ignore, pin, channel, and dependency preferences
- adopted local-addon snapshot archives

Without that state, the product cannot safely answer “what is tracked”, “what source should update
this package”, or “what exact package set should be reproduced on another machine”.

## Backend Rule

Managed addon state is resolved through one runtime-owned backend choice.

Current backends:

- `app-data`: default desktop behavior; state lives in platform app-data keyed by installation
  identity
- `sidecar`: explicit portable behavior; state lives under `Interface/AddOns/.hearthsync`

The CLI exposes that choice globally through:

```text
--addon-state-storage app-data|sidecar
```

That runtime-level switch is intentional.
Backend choice should stay centralized instead of leaking into separate per-feature path flags.

Concrete operator consequence:

- scan-only commands do not create `Interface/AddOns/.hearthsync`
- managed addon commands running on the default `app-data` backend do not create it either
- `Interface/AddOns/.hearthsync` appears only for the explicit `sidecar` backend or for explicit
  bundle sidecar metadata output such as `.hearthsync/bundles/<bundle-id>/`

## Current Managed-State Scope

The current addon-state backend covers:

- addon registry
- addon lock
- addon policy
- adopted snapshot archives

The current backend intentionally does not yet migrate every other sidecar-style artifact.

Notably, bundle unpack sidecar metadata still has its own explicit portability role under:

```text
Interface/AddOns/.hearthsync/bundles/<bundle-id>/
```

That is a separate current design choice, not a contradiction in addon-state defaults.
By contrast, bundle export and the `bundle addon-plan` / `bundle addon-apply` shortcut flows now
still resolve the installation's active tracked addon state through the configured runtime backend
(`app-data` or `sidecar`) instead of silently forcing desktop-default app-data semantics.

## Product UX Guidance

### CLI

- default CLI behavior should preserve a “recognize first” posture for scan-only commands
- commands that enter tracked addon workflows should rely on the configured managed-state backend
- help text should describe defaults in terms of the configured backend, not hard-code one path in
  every command

### Future Desktop UI

- installation discovery should feel available before any tracked-addon commitment
- entering tracked addon management should make the managed-state model visible instead of hiding it
- backend choice should appear as one product/runtime setting, not as scattered feature toggles
- the UI should not imply that “no client-tree mutation” means “the product is fully stateless”

## Suggested Mental Model

Use the following product sentence as the default framing:

> HearthSync can inspect a WoW installation without claiming ownership of it, but once the user
> asks HearthSync to track and synchronize addons, HearthSync needs persistent managed state.

That framing keeps both halves true:

- NewBeeBox-style recognition expectations remain valid
- tracked sync remains technically honest

## Immediate Consequences

- “Do we need `.hearthsync`?” is no longer a yes/no question
- the real question is “which backend should managed addon state use by default?”
- the current answer is “default to app-data, keep sidecar as an explicit portable backend”
- default desktop usage no longer needs client-tree `.hearthsync` just to keep tracked addon
  registry, lock, or policy state

## Follow-Up

Future work may extend this note when:

- desktop settings expose backend choice directly
- bundle sidecar metadata is reconsidered for the same runtime-owned backend model
- migration/import UX needs an explicit transition from scan-only posture into managed mode
