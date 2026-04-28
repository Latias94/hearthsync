# WoW Addon Sync Core Design

## Status

Active on 2026-04-19.

This workstream is the architecture source of truth for the reusable product core.
The CLI workstream remains active, but it should focus on command surface, UX wording, and output
behavior on top of this core.
Author-package-style configuration sync also stays in this workstream.
Portable sync of `WTF`, fonts, interface assets, curated addon payloads, and transfer-progress
contracts depends on the same planner, backup, rollback, and app-task boundaries, so it does not
justify a separate top-level workstream before a real desktop delivery stream exists.

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

## Product Interaction Modes

The product should distinguish between scan-only posture and managed addon mode instead of treating
every installation-facing action as the same kind of ownership.

- scan-only flows discover, inspect, validate, or preview WoW content without establishing tracked
  addon truth for that installation
- managed addon flows depend on persistent HearthSync-owned state for tracked package identity,
  mutable policy, reproducibility, and adopted local snapshots

The full product note for that boundary lives in
`scan-only-vs-managed-mode.md`.

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
- optional external-helper capability now has an explicit runtime boundary, but no concrete helper
  backend exists yet
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

`core::app::StableAppServices` is the intended stable frontend root.
It should own:

- runtime defaults and injected provider/helper policy
- app-facing request and result contracts
- task entrypoints, progress shapes, and cancellation expectations
- orchestration rules that frontends should not rebuild locally

`core::app::ExtendedAppServices` is the broader app extension root for less-stable flows such as
addon-index, addon-lock, and bundle-addon-lock operations. It should compose
`StableAppServices` explicitly instead of implicitly acting as the stable API surface itself.

Stable app request contracts should also own runtime-backed default injection where the caller-
visible contract depends on it.
Backup destination defaults, bundle output defaults, and external-package source-platform defaults
should not be reimplemented ad hoc in each service wrapper or in CLI orchestration.
Those defaults belong to the app boundary because they are part of how a frontend experiences the
operation contract, not just an internal filesystem detail.

The same ownership rule applies to installation discovery and other thin frontend-facing input
projection.
If scan-root policy, host-platform injection, or the last small step from app-owned installation
values to domain installation inputs still lives inside a service wrapper, the service remains a
policy owner instead of a boundary caller.
Runtime or request-side app helpers should own that normalization so services do not keep
rebuilding the same domain inputs for installation scan/inspect/resolve, addon inventory reads,
addon-lock reads or plans, or bundle preview planning.

Runtime-backed mutation requests should follow the same rule all the way to the domain boundary.
If a request needs runtime defaults before becoming a domain mutation input, the request contract
should expose one normalized projection helper instead of leaving each service to coordinate
`apply_runtime_defaults(...).into()` as a repeated two-step protocol.

The same principle applies to provider capability configuration.
If the frontend needs to configure default addon acquisition behavior such as download cache
location or retry policy, that configuration should use app-owned runtime value types rather than
provider-domain structs.
Custom provider injection may still exist as an internal crate seam for tests or specialized
composition, but it should not remain part of the stable frontend runtime contract.
The stable runtime contract should make it explicit whether the app is using configurable
default-provider options or a fully internal custom provider implementation.

The same ownership rule applies to persisted runtime settings.
If the frontend needs addon-state backend choice, addon download cache location, or no-validator
HTTP freshness policy to survive across invocations, that persistence belongs to `core::app`
rather than to CLI-only flag glue.
The current shared backend stores selected runtime overrides under app-data
`settings/runtime.toml`, the CLI exposes `settings inspect|set|reset` on top of that backend, and
one-shot CLI flags remain ephemeral overlays on the current process instead of becoming a second
persistence path.

Helper capability reporting follows the same rule.
The selected helper strategy belongs to app runtime state, not to bundle-planner DTOs.
Public plan results may expose helper strategy as frontend-facing status, but the planner should not
invent or own that capability state itself.
Runtime capability reporting should also distinguish:

- external-helper policy requested by the frontend
- whether an external helper is currently available
- which helper strategy is actually active for the current runtime

That separation prevents future optional helpers from overloading one enum with both desired
policy and actual execution state.

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

Default public planning is intentionally logical and conservative.
If a target path already exists and the planner has not already proven a deterministic `add` or
`preserve`, public plan treats that entry as a replace candidate without opening archive readers,
previewing Lua rewrites, or byte-comparing the current target file.
Exact `skip` versus `replace` resolution for existing targets belongs to prepare/apply paths, where
the system may spend the additional I/O and rewrite-preview cost immediately before execution.

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
- collecting-progress calls that return `TaskRun<TResult>` with one generated `task_id` plus an
  ordered `Vec<TaskProgressEvent>`
- callback-based calls that stream the same `TaskProgressEvent` payloads while polling a caller-
  supplied cancellation closure

Those task contract types should be consumed through `core::app`, so frontend callers do not need
to depend on the lower-level `core::task` module directly just to drive stable app services.

Stable progress event fields are:

- `task_id`
- `task`
- `phase`
- optional `code`
- optional `current`
- optional `total`
- optional `bytes_current`
- optional `bytes_total`
- optional `bytes_per_second`
- `message`

`message` remains the human-readable CLI/display string.
The optional structured fields exist so future GUI code can group progress by task, render
step-level counts, and later consume byte-level transfer updates without string parsing.
Provider-backed addon acquisition now uses this same event shape for real transfer updates too.
Download phases emit `code = DownloadArchive` plus `bytes_current`, `bytes_total`, and
`bytes_per_second` while keeping a CLI-readable `message`, so future GUI work does not need a
second download-specific progress channel.

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

Portable bundle export should also avoid ambient process-state assumptions.
Default output paths, relative output references, and manifest-relative sidecar references such as
bundle addon indexes should resolve from explicit product inputs like runtime defaults,
`manifest_base_dir`, or installation-derived base directories rather than the caller's current
working directory.

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

`ExtendedAppServices` may still expose less-stable app operations for internal and CLI use, but the
explicit first-wave stable service boundary should be the contract future GUI work prefers.
When callers need both surfaces, they should cross that boundary intentionally through
`ExtendedAppServices::stable()` instead of relying on implicit compatibility behavior.

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

Mutable addon policy should now follow that same rule too.
Addon ignore/pin/channel/pre-release/dependency choices should live in a separate
HearthSync-managed addon policy state file instead of leaking into `lock.toml`, and stable callers
should use app-owned addon-policy value types plus first-class inspect/set/remove entrypoints
rather than domain policy enums or ad hoc frontend state. Managed addon state now defaults to
platform app-data storage keyed by installation identity, while sidecar `.hearthsync` remains an
explicit portable backend instead of the default desktop path. Runtime diagnostics should expose the
selected backend generically and, when the caller has already resolved one installation, the exact
managed addon state paths for that installation too. The canonical app-data root should stay
product-named rather than duplicating organization and application segments. Because this remains a
pre-release codebase, the project should prefer one canonical managed-state layout over carrying
forward obsolete path compatibility branches.

Current execution coverage for that policy is intentionally narrower than the stored schema.
Bulk `addon update` and `addon index update` already consume `ignored = true`, and provider-backed
`addon update` also applies basic pin overrides (`pin.file_id` for CurseForge, `pin.version` as a
GitHub tag override) while preserving the tracked `package_id`.
Regular provider-backed `addon update` now also consumes release-channel/prerelease source
selection plus `install_dependencies = true` for the first explicit dependency-installation slice:
missing required CurseForge dependencies are installed as additional tracked packages, while
unsupported source kinds fail explicitly instead of silently treating the policy field as a no-op.
Addon-index update now also consumes that same first `install_dependencies = true` slice, but it
resolves dependencies from the curated index source while still treating those curated source
declarations as authoritative instead of letting user policy override them via pin or
release-channel/prerelease preferences. The execution layer now also reflects that distinction
explicitly through separate provider-backed and indexed-update policy projections instead of
sharing one “everything can read everything” policy accessor surface. Provider-side dependency
resolution now follows the same rule: the contract expresses an explicit dependency-resolution
strategy rather than returning a bare source list whose semantics only exist in comments. Provider-
side dependency support is also advertised through an explicit capability surface, so “unsupported”
and “missing required only” are part of the contract rather than only emerging from validation
errors after execution has already started. That capability now also projects through app-owned
source value payloads, so future GUI work can inspect dependency support from stable addon, index,
and lock results without reaching into provider-domain internals. The app boundary now also owns a
first preflight gate for that capability: addon and addon-index update entrypoints reject
`install_dependencies = true` for unsupported sources before they enter domain update execution.
Indexed update now also reuses provider-level source-family identity during both preflight and
domain matching, so index-package id drift and GitHub asset-name drift no longer force a fallback
when the tracked package still points at the same underlying package family. It also accepts unique
exact display-name continuity as a later fallback, so source-family migration can still preflight
when the curated package name remains stable across tracked package id, stored metadata package
name, addon directory name, or addon title. The curated index schema now also supports explicit
exact `match_package_ids` hints, so index authors can bridge known historical tracked package ids
without adding fuzzier runtime heuristics. Preflight remains intentionally conservative only for
the narrower case where the curated source family itself changes and the index package also omits
explicit `match_package_ids`, stable `addon_directories`, and any exact unique display-name
continuity, so the deeper domain validation path still remains the final correctness backstop.
Curator diagnostics now also treat those exact-hint gaps as structured issues with explicit
severity. Packages that omit both hint types surface one blocking
`missing_exact_identity_hints` issue because that is the remaining case that can still force
preflight down to the deeper domain validation path. Packages that declare only one hint type now
surface advisory `missing_match_package_ids` or `missing_addon_directories` issues instead, so
inspect/validate/GUI consumers can still encourage fuller curation without turning every partial
hint into a hard failure. Validation now keys off blocking warning count rather than total warning
count, which keeps exact missing-bridge failures non-zero while still exposing softer curator
follow-up work in the same structured result.
Curator authoring help should follow the same contract too. Rather than adding weaker runtime
matching rules, addon-index curation should reuse the existing preflight matching order to explain
one current local mapping and propose missing exact hints from it. The new suggestion surface does
that directly: it resolves local-archive index sources through the same index-path canonicalization
used by install/update, matches one tracked package through the same preflight rule order, and
then suggests only the missing `match_package_ids` and `addon_directories` additions that would
make that mapping explicit next time. Ambiguous or absent local matches stay structured results
instead of turning the helper into another all-or-nothing validator.
Initial authoring should follow the same principle too. When no curator index exists yet, the tool
should bootstrap one from tracked package state instead of making operators hand-write package
source references from scratch. The new scaffold surface does that from the tracked addon registry:
it preserves existing curated metadata when already available, falls back to tracked addon
directory/title/version data only where necessary, emits addon directories directly from tracked
package contents, and only injects `match_package_ids` when the preserved curated package id differs
from the tracked local package id. It also fails closed when no tracked registry exists yet, rather
than inventing source references from an untracked addon folder scan.
That still leaves one real bootstrap gap for existing manual installs: if a machine already has
usable addon directories but no tracked registry, addon-index scaffold/suggest cannot help yet.
That gap still belongs in `wow-addon-sync-core`; it is not a new workstream because it cuts across
addon source identity, registry semantics, CLI/app product flow, and future GUI reuse at the same
time.
The bootstrap rule should stay explicit and honest:

- do not silently sweep the entire `Interface/AddOns` directory into one or more tracked packages
- require explicit addon-directory selection from the operator
- if multiple addon directories belong to one package, require an explicit tracked `package_id`
- snapshot the selected directories into a real local archive and record that archive as the
  tracked source
- do not invent remote provider identity when the current machine no longer knows it
- do not add a self-referential "local addon directory" source kind whose update behavior only
  points back at the mutable installation itself

That explicit adopt path keeps the source model honest while still unblocking the real operator
workflow:

1. adopt explicit untracked local addon directories into tracked state
2. scaffold or suggest curator data from the new tracked registry
3. later replace the local snapshot source with a real curated or provider-backed source when known
   through an explicit relink step rather than silent source inference

The first implementation of that follow-up step is intentionally narrow.
`addon relink` prepares one new source, requires the prepared addon-directory set to match the
currently tracked package exactly, and then rewrites only the tracked registry source.
It does not rewrite live AddOns content, and it clears old package metadata rather than carrying
forward stale curator/source details that may no longer describe the new source truthfully.

The curator-aware follow-up now builds on that same rule set instead of inventing a separate
mutation model.
`addon index relink` resolves one package from a curated index, prepares its resolved source,
requires exact addon-directory parity against one tracked package, and then rewrites registry
source plus curated metadata together. It still does not rewrite live AddOns content, but unlike
generic relink it intentionally allows "metadata attach only" when the source already matches and
the missing step is simply attaching truthful curated identity.

The higher-level bulk curator workflow now follows the same rule set too.
`addon index attach` reuses the same suggestion-style tracked-package matching order across one
selected index scope, prepares each resolved source only to prove exact addon-directory parity, and
then returns a structured ready/blocked/skipped review result before any registry mutation occurs.
Execution is deliberately fail-closed: the command may preview mixed states, but it only writes
registry source plus curated metadata when every selected package is attachable, so future GUI work
can present operator review without inheriting partial-write semantics.

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

- which first concrete external-helper backend, if any, is worth integrating after the runtime
  boundary is explicit
- how much additional archive compatibility hardening is needed before real public release
- whether a workspace split becomes worthwhile after the planner and app-contract refactors are done
