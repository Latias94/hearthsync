# Architecture Hardening Review - 2026-04-28

## Status

Completed review record for the pre-GUI fearless-refactor phase.

This review stays inside `docs/workstreams/wow-addon-sync-core` instead of creating a new top-level
workstream. The findings affect shared addon mutation, index update, provider acquisition, config
sync, and app-task contracts. Splitting them into a parallel workstream would duplicate the current
core architecture source of truth.

## Verification Snapshot

- `cargo fmt -- --check` passes.
- `cargo nextest run` passes with 573 tests.
- `cargo clippy --all-targets -- -D warnings` passes.

The original failing clippy run was used as a refactor signal rather than suppressed. Mechanical
warnings were fixed directly, and meaningful `too_many_arguments` diagnostics were converted into
request/context objects across addon dependency collection, package preparation, mutation
execution, lock-source preparation, task progress, index attach result creation, and bundle plan
assembly.

## Current Architecture Read

The codebase already has a useful reusable-core shape:

- CLI commands build app-owned requests.
- `core::app` exposes stable and extended service entrypoints.
- addon, bundle, external-package, backup, and Lua rewrite logic live below the app boundary.
- backup, rollback, path-safety, archive-safety, and cross-platform collision checks are widely
  covered by tests.
- author-package-style configuration sync reuses the bundle planner and execution pipeline instead
  of creating a second file-copy engine.

That direction is sound. The remaining risks are not "rewrite everything" risks. They are
transaction-boundary, policy-ordering, public-contract, and module-ownership risks that should be
fixed before the future `egui` surface depends on them.

## Findings

### F1 - Dependency Install Failure Can Bypass Addon Update Rollback

Relevant code:

- `src/core/addon/execution.rs`
- `src/core/addon/index/operations.rs`

The update flow first updates selected primary packages through `update_prepared_packages_task`.
When that succeeds, missing dependency packages are installed in the `Ok` branch. Dependency
installation currently uses `?` from inside that branch.

Risk:

- a primary addon can be updated successfully
- a dependency install can then fail
- the error can leave through the success branch instead of being normalized through the same
  rollback path as the primary update failure

Desired behavior:

- every mutating step after the backup point participates in one transaction-like outcome
- primary update failure and dependency install failure both restore from the backup when rollback is
  available
- the returned error explains whether rollback succeeded, failed, or was unavailable

### F2 - Ignored Index Packages Can Be Prepared Before Being Skipped

Relevant code:

- `src/core/addon/index/operations.rs`

Bulk `addon index update` preflights a package match, resolves the source, prepares the package, and
only then checks whether policy marks the matched package as `ignored`.

Risk:

- ignored packages can still trigger network download and archive preparation
- bulk update progress may report work that the user's policy said to skip
- future GUI progress would have to explain confusing "downloaded then ignored" behavior

Desired behavior:

- if preflight matching is enough to identify the tracked package, the ignored policy should be
  applied before source resolution and package preparation
- explicit named update may still override `ignored`
- deferred or ambiguous matching cases should fail closed or clearly report why preparation was
  required

### F3 - Config Sync App Boundary Is Still Too External-Package-Shaped

Relevant code:

- `src/core/app/config.rs`
- `src/core/app/request/config.rs`
- `src/core/app/response/config.rs`

The first-class `config` surface exists, but the response types are aliases of external-package
results and most request conversion is direct forwarding.

Risk:

- GUI code will either display external-package vocabulary for config sync or create its own mapping
  layer outside `core::app`
- future config-specific UX decisions will be harder because the stable contract has no product-owned
  config model

Desired behavior:

- keep the shared engine underneath
- expose config-owned app DTOs that use config-sync vocabulary
- convert between config DTOs and external-package domain/app internals at the service boundary

### F4 - Addon Provider Module Has Too Many Responsibilities

Relevant code:

- `src/core/addon/provider/mod.rs`

The provider module owns trait definitions, default provider composition, source materialization,
cache behavior, remote validation, repair behavior, URL helpers, and provider-specific glue.

Risk:

- adding one provider or cache behavior requires understanding unrelated responsibilities
- clippy `too_many_arguments` in provider repair/refresh helpers reflects missing context objects
- GUI diagnostics and settings will need clearer provider/cache/capability concepts

Desired behavior:

- keep public provider semantics stable inside the crate
- split cache metadata and repair into a cache-focused module
- split materialization/download orchestration from source-family adapters
- introduce small context structs where function argument lists are carrying implicit subsystem
  ownership

### F5 - Long-Running Task API Is Not Yet a GUI Contract

Relevant code:

- `src/core/task/mod.rs`
- `src/core/app/task_support.rs`
- `src/core/app/stable.rs`
- `src/core/app/extended.rs`

The lower layers already support callbacks and cancellation, but the stable service entrypoints
mostly return `TaskRun<T>` after completion. Callback variants are primarily crate-internal or
service-internal.

Risk:

- `egui` integration may have to depend on internal service methods
- GUI cancellation and live progress could fork away from CLI task behavior
- progress event shape is good, but task ownership and streaming are not yet product-level
  contracts

Desired behavior:

- promote one app-owned long-task contract that supports cancellation and live progress
- make CLI and GUI consumers of the same task runner semantics
- keep collected-progress helpers as convenience wrappers, not the only stable path

### F6 - Clippy Is Not Yet a Quality Gate

Relevant command:

- `cargo clippy --all-targets -- -D warnings`

The current clippy output contains a mix of mechanical cleanup and real boundary pressure.

Risk:

- new refactor work can add more warnings without noticing
- real design smells such as long argument lists stay blended with mechanical style noise

Desired behavior:

- clean or intentionally allow the current warnings in small slices
- use context structs for meaningful `too_many_arguments` cases instead of blanket allows
- make new code keep clippy clean once the baseline is manageable

## Refactor Sequence

### H1 - Transaction-Complete Addon Mutation

Unify primary update and dependency installation under one rollback-aware execution result.

Implementation direction:

- extract a helper that executes all post-backup addon mutations
- return one `UpdatedAddonPackageResult` only after every package mutation succeeds
- route dependency install errors through the same rollback/error normalization as update errors
- add regression tests where dependency install fails after the primary update writes files

### H2 - Policy-First Index Update Planning

Move ignored-policy evaluation as early as the available match evidence allows.

Implementation direction:

- use preflight match output to load package policy before source preparation
- skip bulk ignored packages before network or archive work
- preserve explicit named update override behavior
- add tests that assert ignored packages do not call the provider

### H3 - Config-Owned App Contracts

Make `config` a product-owned stable app surface while retaining the shared external-package engine.

Implementation direction:

- replace response type aliases with config-specific structs
- map external-package results into config results at `ConfigService`
- keep CLI output behavior stable while using config vocabulary internally
- document that external-package is an implementation engine for config sync, not the user-facing
  contract

### H4 - Provider Module Decomposition

Split provider responsibilities along cache, materialization, validation, and source-adapter lines.

Implementation direction:

- move cache sidecar, repair, freshness, and validator code into focused modules
- introduce provider operation context structs for repair/refresh/materialization helpers
- keep behavior unchanged during the split
- rerun provider tests after each slice

### H5 - App-Owned Live Task Contract

Expose a frontend-ready task API above the existing progress event model.

Implementation direction:

- design one app-level runner or task handle that accepts cancellation and progress callbacks
- keep `TaskRun<T>` as a convenience collected-progress result
- ensure stable and extended services do not require GUI callers to reach into internal methods
- keep progress events machine-readable and avoid CLI-text parsing

### H6 - Clippy Baseline Cleanup

Turn clippy into a practical refactor guardrail.

Completed: `cargo clippy --all-targets -- -D warnings` now passes. Mechanical warnings were
removed directly, and argument-heavy helpers now carry explicit request/context objects instead of
implicit parallel parameter lists.

Implementation direction:

- fix mechanical warnings directly
- convert meaningful long-argument helpers into context structs
- add narrow `allow` attributes only when a warning is intentional and documented
- target `cargo clippy --all-targets -- -D warnings` as the end-state gate

## Non-Goals

- no GUI implementation in this slice
- no remote provider compatibility expansion beyond what the refactor requires
- no legacy compatibility preservation for pre-release app contracts
- no separate config-sync engine

## Open Questions

- Should dependency installation stay coupled to update execution, or should it become a prepared
  mutation phase with a richer transaction object?
- Should the first GUI task contract be callback-based only, or should it expose a channel/stream
  abstraction that maps naturally into `egui` polling?
- Clippy cleanup has now completed after provider decomposition, with the remaining meaningful
  argument-list warnings absorbed into focused request/context objects.
