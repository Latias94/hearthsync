# WoW Addon Sync Core Design

## Status

Active on 2026-04-19.

This workstream is the architecture source of truth for the reusable product core.
The CLI workstream remains active, but it should focus on command surface, UX wording, and output
behavior on top of this core.

## Problem Statement

`hearthsync` is no longer only a CLI experiment.
It is becoming a cross-platform WoW addon and configuration sync product that must support:

- direct CLI use
- a future `egui` desktop frontend
- addon download and update flows
- portable first-party bundle export and apply
- direct import of author-distributed UI packages
- safe migration of `WTF`, fonts, and interface assets across Windows and macOS

The codebase already contains most of the required domain logic, but it still carries prototype-era
behavior in a few critical places:

- some public planning paths still do too much execution-shaped preparation
- direct author-package import previously defaulted to a merge-first profile that is unsafe for
  real setup-package migration
- `core::app` still behaves more like a forwarding layer than a fully-owned application contract

If those boundaries are not corrected now, CLI-specific or prototype-specific assumptions will leak
into the future desktop integration path.

## Goals

- define one reusable Rust core for CLI and future desktop callers
- keep the product centered on WoW semantics instead of raw file-copy behavior
- converge bundle apply and author-package import on one planning and safety model
- make destructive operations previewable, backup-aware, and rollback-oriented
- delete transition code aggressively once the cleaner path exists

## Non-Goals

- immediate migration to a multi-crate workspace
- full async conversion before runtime and task boundaries are stable
- GUI implementation in this workstream
- broad launcher-format compatibility before the core contracts are clean

## Product Model

The product treats a WoW installation as structured content groups rather than one flat directory.

Primary install groups:

- `Interface/AddOns`
- `WTF/Config.wtf`
- `WTF/Account/<account>/SavedVariables`
- `WTF/Account/<account>/<server>/<character>`
- `Fonts`
- selected `Interface/*` subtrees outside `AddOns`

Portable bundle groups:

- `addons`
- `wtf_common`
- `wtf_characters`
- `fonts`
- `interface_assets`
- `metadata`

This model intentionally stays close to the real WoW filesystem while giving the planner explicit
resource-group semantics.

## Current Architecture Snapshot

The repository already has useful core subdomains:

- `install`: installation discovery and inspection
- `backup`: backup, list, and restore workflows
- `bundle`: pack, inspect, plan, unpack, author-package import, and addon-lock sidecar flows
- `addon`: provider-backed install, update, remove, lock, and index flows
- `lua_patch`: targeted rewrite rules for selected `SavedVariables`
- `manifest`: first-party bundle manifest schema
- `app`: frontend-facing services, requests, results, runtime policy, and task wrappers

Current strengths:

- resource-group-aware planning and apply execution
- one backup and rollback boundary for bundle and addon-lock mutations
- direct external-package plan/apply path without mandatory temporary bundle repacking
- app-facing request and result contracts are already moving away from raw domain leakage
- stable app/runtime contracts now also own platform and flavor identity through app-owned
  `HostPlatformValue` and `WowFlavorValue`

Current architectural constraints:

- public planning still keeps too much execution-shaped preparation internally
- `core::app` services still need stronger contract ownership and fewer thin-forwarder behaviors
- optional external-helper capability is not yet modeled as an explicit boundary
- archive compatibility and portability coverage still need more real-world hardening

## Target Architecture

The target product shape is a layered application core:

```text
CLI / Desktop
  -> core::app services and task wrappers
    -> core domain modules
      -> provider, archive, and filesystem adapters
```

Short-term source layout remains:

```text
src/
  lib.rs
  main.rs
  cli/
  core/
```

The project does not need a workspace split yet.
The first requirement is to make the logical boundaries real in APIs, requests, results, and
runtime policy ownership.

## Core Boundaries

### Library Boundary

`lib.rs` should expose reusable core modules.
`main.rs` should remain a thin shell that parses CLI arguments and renders output.

### Application Boundary

`core::app::HearthSyncApp` is the intended frontend root.
It should own:

- runtime defaults and injected provider/helper policy
- app-facing request and result contracts
- task entrypoints, progress shapes, and cancellation expectations
- orchestration rules that frontends should not rebuild locally

Stable app request contracts should also own runtime-backed default injection where the caller-
visible contract depends on it.
Backup destination defaults, bundle output defaults, and external-package source-platform defaults
should not be reimplemented ad hoc in each service wrapper or in CLI orchestration.
Those defaults belong to the app boundary because they are part of how a frontend experiences the
operation contract, not just an internal filesystem detail.

The same principle applies to provider capability configuration.
If the frontend needs to configure default addon acquisition behavior such as download cache
location or retry policy, that configuration should use app-owned runtime value types rather than
provider-domain structs.
Custom provider injection may still exist as an internal crate seam for tests or specialized
composition, but it should not remain part of the stable frontend runtime contract.
The stable runtime contract should make it explicit whether the app is using configurable
default-provider options or a fully internal custom provider implementation.

Helper capability reporting follows the same rule.
The selected helper strategy belongs to app runtime state, not to bundle-planner DTOs.
Public plan results may expose helper strategy as frontend-facing status, but the planner should not
invent or own that capability state itself.

The same ownership rule applies to response projection.
Domain-to-app result mapping may still exist internally, but stable app result types should not
publish public `From<domain>` trait surfaces that encourage frontend callers to couple themselves
to domain result shapes.

### Domain Boundary

Domain modules remain responsible for WoW semantics:

- installation and flavor rules
- resource-group semantics
- account and character targeting
- addon identity and lock semantics
- bundle manifest rules
- rewrite targeting rules for selected configuration files

### Infrastructure Boundary

The following dependencies belong behind explicit ports or adapters:

- addon providers
- archive readers and writers
- filesystem mutation helpers
- optional external migration helpers

## Direct Author-Package Import Model

Users do not only need first-party `manifest.toml` bundles.
They also need to import author-distributed setup packages that may contain:

- `Interface/AddOns`
- `WTF`
- `Fonts`
- interface assets such as textures or materials

The direct import path therefore does the following:

1. inspect an archive or directory
2. classify files into product resource groups
3. normalize the input into source entries with stable bundle-like paths
4. reuse the same planning and execution pipeline as first-party bundles

This keeps the system honest:

- one planning model
- one execution model
- one safety model

Explicit normalized-bundle export remains valid for workflows that intentionally want a reusable
first-party archive, but it is no longer the mandatory hot path for direct import.

## Default Apply Semantics for Author Packages

Direct author-package import must not default to merge-first behavior.
Real-world UI packages are usually intended to replace stale local addon, font, and interface
content, while `WTF` requires more conservative handling.

The current default profile is:

- `create_backup = true`
- `addons = mirror`
- `wtf_common = share`
- `wtf_characters = replace_selected`
- `fonts = mirror`
- `interface_assets = mirror`

Rationale:

- author packages should not leave stale addons mixed into the target installation by default
- fonts and interface assets behave more like synced payload groups than append-only data
- common `WTF` is risky to overwrite blindly and should default to additive sharing
- character `WTF` should stay explicit and target-bound, not global

CLI flags may override any group, but unspecified groups must inherit this shared profile rather
than falling back to `merge`.

## Planning and Execution Model

Mutating operations should converge on the same structure:

1. inspect and validate target
2. build an explicit logical plan
3. create a backup checkpoint when required
4. prepare execution inputs
5. execute deterministic operations in a stable order
6. summarize the result or roll back on failure

The remaining refactor priority is to make public plan APIs stay at step 2 while keeping steps 4
and later internal.

## Public Planning Contract

`bundle plan` and `external-package plan` are public dry-run contracts.
They may expose logical preview data that frontends or automation need to explain intent:

- manifest-derived group policies
- selected target accounts and character mappings
- logical operations with `group`, `wtf_scope`, `action`, normalized archive identity, and target destination
- plan summaries and helper strategy
- external-package analysis data that explains how an author package was normalized

They must not expose execution-only payloads or staging details such as:

- per-operation rewrite vectors or rewrite-applied flags
- source-entry maps, apply-source variants, or source-materialization paths
- temporary staging paths or byte-materialization state
- backup checkpoint internals or rollback bookkeeping

If execution later needs more data, that data should be projected from a smaller logical preview
result into an internal execution payload at the apply boundary rather than being added to the
public plan model.

## Stable Task Contract

Long-running app-facing operations share one stable wrapper contract under `core::app`.
This applies to addon install, update, and remove; addon-index install and update; addon-lock
apply; bundle apply; external-package analyze, plan, and apply; and backup restore.

The stable entry shapes are:

- direct calls that return only `AppResult<TResult>` and run with no cancellation plus a no-op
  progress sink
- collecting-progress calls that return `TaskRun<TResult>` with an ordered `Vec<TaskProgressEvent>`
- callback-based calls that stream the same `TaskProgressEvent` payloads while polling a caller-
  supplied cancellation closure

Those task contract types should be consumed through `core::app`, so frontend callers do not need
to depend on the lower-level `core::task` module directly just to drive stable app services.

Stable progress event fields are:

- `task`
- `phase`
- `message`

For successful long-running tasks, the frontend-facing expectation is:

- the first reported phase is `Preparing`
- the last reported phase is `Completed`
- intermediate phases are task-specific and may include `Planning`, `BackingUp`, `Executing`, or
  `Verifying`

Cancellation or hard failure may stop the stream before `Completed`, but callers should still be
able to reason about the task using the same event shape and `TaskKind` plus `TaskPhase`
semantics regardless of whether they consume direct, collected, or callback-based entrypoints.

## Cross-Platform Strategy

Primary supported targets:

- Windows
- macOS

Key concerns:

- case-insensitive path collisions on Windows and default macOS targets
- archive path normalization and encoding
- flavor compatibility
- account and character remapping
- author packages produced on one platform and applied on the other

The product should prefer deterministic refusal over ambiguous or destructive guesses.

## Current Refactor Sequence

### R0 - Workstream Rebaseline

This workstream now owns the refactor plan, milestones, and architecture decisions.

### R1 - Author Package Default Semantics

The first bounded fearless-refactor slice corrects direct author-package defaults and makes core
and CLI share one profile.
This intentionally changes behavior because the old merge-first default was not aligned with the
product goal.

### R2 - Logical Planner Boundary

The next slice removes more execution-shaped preparation from public planning so `bundle plan` and
`external-package plan` become cleaner dry-run contracts with an explicit logical-only payload.

### R3 - Stable `core::app` Contracts

After the planner boundary is smaller, the next refactor should stabilize app service ownership,
task expectations, and runtime policy injection for future `egui` work.

## First-Wave GUI-Stable Services

The first frontend-stable service set should stay deliberately small.
It covers the user-facing flows needed for the initial WoW sync product without forcing the GUI to
depend on every advanced or power-user API on day one.

First-wave GUI-stable services:

- `InstallationService`
- `AddonService`
- `BundleService`
- `ExternalPackageService`
- `BackupService`

These services cover:

- installation discovery and inspection
- addon search, install, update, remove, and inventory
- first-party bundle inspect, pack, dry-run plan, and apply
- direct author-package analyze, bundle export, dry-run plan, and apply
- backup create, list, and restore

Services that remain app-level but are not part of the first-wave GUI-stable contract:

- `AddonIndexService`
- `AddonLockService`

Those capabilities are still valuable, but they represent curation and reproducibility workflows
that can evolve after the first desktop-facing sync surface is stable.

`HearthSyncApp` may still expose all app services for internal and CLI use, but the explicit
first-wave stable service boundary should be the contract future GUI work prefers.

The first shared stable value object under that boundary is the resolved installation shape.
`InstallationService::resolve` should return an app-owned resolved installation value, and other
app service requests should consume that same value instead of exposing domain
`DetectedFlavorInstallation` directly to frontend callers.

The next shared stable value objects under that boundary are the bundle/external-package apply
strategy inputs.
Target-account selection, character remapping overrides, and author-package default policy
overrides should stay app-owned so CLI and future `egui` callers do not need domain
`BundleApplyMappings` or manifest `ApplyDefaults` types just to drive the stable bundle and
external-package service flows.

Stable addon package metadata should follow the same rule.
Addon install requests and tracked-package metadata results should use one app-owned metadata value
so stable addon callers do not need domain `AddonPackageMetadata` to pass curated metadata through
the frontend boundary or read it back from addon inventory results.

Full manifest payloads should follow that same ownership rule too.
Pack requests and bundle/external-package result payloads should use one app-owned manifest value
tree so stable callers do not need domain `BundleManifest` just to submit or consume a complete
manifest shape across the app boundary.

Stable platform and flavor identity should follow the same ownership rule.
`AppRuntime`, installation resolve and inspect requests, resolved installation results, and bundle
or external-package source metadata should use app-owned `HostPlatformValue` and
`WowFlavorValue` so frontend callers do not need install-domain enums just to express host
defaults, source compatibility, or an installation flavor selection.

The same rule now also applies to manifest mapping rules and installation-health payloads.
Stable app-facing manifest values use `CharacterMappingModeValue`, and stable installation
inspection results use `HealthStatusValue`, so frontend callers no longer need install-domain or
manifest-domain enums for those states either.

With those value boundaries in place, the remaining `R3` work is no longer another broad DTO
migration. The next stability decisions are mainly about service behavior ownership:
thin-forwarder normalization, progress expectations, and runtime capability injection.

## Open Questions

- which app services should be declared GUI-stable first
- what minimal external-helper capability boundary is worth keeping
- how much additional archive compatibility hardening is needed before real public release
- whether a workspace split becomes worthwhile after the planner and app-contract refactors are done
