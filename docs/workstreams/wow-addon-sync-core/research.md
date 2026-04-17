# WoW Addon Sync Core Research

## Scope

This note summarizes the product and architecture findings that justify a reusable core workstream.
It is not a reverse-engineering dump.
It is a product-shaping summary for future implementation work.

## WoW Installation Findings

Relevant resource groups continue to be:

- `Interface/AddOns`
- `WTF/Config.wtf`
- `WTF/Account/<account>/SavedVariables`
- `WTF/Account/<account>/<server>/<character>`
- `Fonts`
- selected `Interface/*` assets outside `AddOns`

Important detail:

- `WTF/Account/SavedVariables` must not be treated as a playable account

The current code already reflects that correction in installation discovery and bundle planning.

## WTF Migration Findings

Reference-product research showed that `WTF` is not one flat replace-or-merge switch.
Practical migration needs:

- account-aware overwrite scope
- character-aware remapping
- byte-safe handling for some `SavedVariables`
- explicit distinction between shared account config and character config
- different policies such as share, sync, preserve, or replace-selected

The current HearthSync code has already moved in that direction through bundle apply policies and Lua rewrite rules.

## Addon Delivery Findings

Addon acquisition and addon installation are different concerns.
The reference product appears to treat them as separate layers:

1. choose acquisition path
2. prepare or obtain an archive
3. feed one archive extraction backend

This is important for HearthSync because future acquisition modes may include:

- direct provider download
- curated index resolution
- bundle-embedded source archive
- local cache hit
- helper-assisted diff delivery

The core should therefore model archive acquisition as a pluggable backend rather than a hard-coded side effect inside install commands.

## Third-Party Package Findings

Users do not only want first-party `manifest.toml` bundles.
They also want to consume author-provided setup packages that may arrive as:

- a zip with `AddOns`, `WTF`, and `Fonts`
- a zip with `Interface/AddOns` and a partial `WTF`
- a folder tree exported from another machine

The current HearthSync bundle engine is strong for first-party bundles but does not yet normalize those third-party layouts into the same internal planning model.

## Current HearthSync Strengths

- clear core module split between install, addon, bundle, backup, manifest, and Lua rewrite
- explicit bundle planning and execution separation
- operation-level backup and rollback for bundle apply and addon-lock apply
- derived addon lock with content hashes for drift detection
- test coverage around current sync semantics

These strengths make a larger refactor feasible now.

## Current Architectural Gaps

### 1. Binary-first crate shape

The repository still starts from a binary root.
That shape is not ideal for a future desktop application that should call reusable core services.

### 2. Missing task contract

Long-running operations still return final results only.
They do not yet expose a stable model for:

- progress
- warnings
- cancellation
- shared task execution lifecycle

### 3. Provider coupling

Provider networking is still direct inside core flows.
That makes UI integration, testing, and future async migration more expensive.

### 4. Restore safety gap

Backup restore is functional but still closer to a replay operation than a transaction-safe restore pipeline.
It should eventually gain stronger validation, staging, and failure semantics.

### 5. Third-party package import gap

There is not yet a general analyzer that can turn arbitrary UI package layouts into the same apply-plan model used by first-party bundles.

### 6. Planning purity gap

Bundle planning is no longer one undifferentiated phase.
It now has an internal logical-planning stage followed by execution preparation, but the public
plan path still reads bundle entry bytes, previews Lua rewrites, and compares candidate output
with target files to preserve today’s preview semantics.
That means dry-run performance and future GUI previews still depend on execution-shaped work.
One concrete improvement is already in place: per-entry `rewrite_applied` no longer leaks through
public apply operations and remains an execution-preparation concern only.

### 7. External-package bridge separation

The old mandatory temporary-bundle bridge has now been deleted from direct external-package plan
and apply.
The remaining architectural requirement is to keep explicit normalized-bundle export as a separate
workflow so it does not leak back into the direct sync hot path.

### 8. App-service thinness

`core::app` exists, but most current services are still light forwarding wrappers.
The service layer does not yet own provider injection, cache policy, retry policy, or a richer task
execution context.

## Research Conclusions

- The product has outgrown a CLI-only architecture lens.
- The existing codebase is already strong enough to support a bounded fearless refactor.
- The first bounded cleanup has now deleted the temporary-bundle bridge from direct external-package flows, proving the normalized-source path is viable.
- The next safest move is to keep shrinking execution-shaped work out of public planning APIs before adding more product surface area.
- Third-party UI package import should be treated as normalization into the same core planning model, not as a parallel installer.
- The explicit temporary bundle export path can remain, but it should become an optional product capability rather than an internal requirement for direct sync flows.
