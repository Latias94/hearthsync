# WoW Addon Sync Core TODO

## Current Focus

The current priority is to turn the existing implementation into a reusable application core while
hardening real-world addon and WTF compatibility before more surface area is added.

The active sequence is:

1. expose a reusable library entrypoint
2. move architecture ownership to this workstream
3. remove the temporary-bundle bridge from direct external-package plan and apply [completed on 2026-04-17]
4. harden real-world import compatibility and delete duplicated classification paths
5. harden portable path semantics for external packages and addon indexes
6. make bundle planning a cleaner logical boundary
7. harden provider, task, and account-discovery contracts for future GUI reuse

## Phase 0 - Workstream Bootstrap

- [x] Create the core workstream
- [x] Record reusable core architecture goals
- [x] Record research summary and current gaps
- [x] Repoint CLI workstream notes to the new core source of truth

## Phase 1 - Library Boundary

- [x] Expose a package library entrypoint
- [x] Decide the first stable top-level public API surface
- [x] Add library-level smoke tests for reusable entrypoints
- [ ] Reduce binary-specific assumptions in module ownership
Current candidate: `core::app::HearthSyncApp` is now the first explicit top-level application API for
desktop or CLI callers, with `InstallationService`, `BundleService`, `ExternalPackageService`,
`AddonService`, `AddonIndexService`, `AddonLockService`, and `BackupService` created from one shared
runtime policy instead of every caller constructing unrelated facades ad hoc.

## Phase 2 - Task Model

- [ ] Define task input and result conventions
- [x] Define progress event model
- [x] Define cancellation checks or token model
- [x] Introduce task wrappers for bundle apply and addon lock apply
- [x] Introduce task wrappers for addon install, update, and remove
- [x] Introduce task wrappers for backup restore
- [ ] Add finer-grained progress beyond phase-only events for long file-oriented operations
- [ ] Add cancellation checks inside long-running execution loops instead of only between phases
Current coverage: bundle apply and external-package apply now emit per-operation `executing` progress and honor cancellation inside the execution loop, but addon and backup flows still need the same treatment where it matters.

## Phase 3 - Provider and Infrastructure Ports

- [x] Introduce addon provider traits
- [x] Move blocking HTTP behind provider ports
- [x] Define archive read and write helpers as infrastructure services
- [ ] Define optional external helper capability boundary
- [x] Add timeout-bounded and truthfully cancellable download behavior for provider-backed addon acquisition
Current coverage: provider-backed addon downloads now reuse one bounded-timeout blocking reqwest
client, stream archives to disk in chunks, stop on task cancellation while writing, and preserve
cancelled semantics without retrying cancelled operations.

## Phase 4 - Import Normalization

- [x] Design analyzer input model for external archives and folders
- [x] Detect addon, WTF, fonts, and interface asset groups from third-party packages
- [x] Produce a normalized import model or temporary internal manifest
- [x] Reuse the same plan and execution pipeline as first-party bundles
- [x] Remove the mandatory temporary first-party bundle bridge from direct external-package plan
- [x] Remove the mandatory temporary first-party bundle bridge from direct external-package apply
- [x] Keep explicit normalized-bundle export as an optional workflow, not an internal dependency
- [x] Replace duplicated addon-root detection with one shared classifier reused by addon archive install and external-package analysis
- [x] Accept addon roots whose `.toc` file name differs from the directory name
- [x] Normalize root-level `WTF/Account/SavedVariables` into the common WTF model
- [x] Add regression coverage for root-level `WTF/Account/SavedVariables` imports
- [x] Reject normalized external-package path sets that would collide on case-insensitive Windows or default macOS targets
Current hardening: normalized external-package path sets that differ only by case are now rejected
before planning or apply would materialize them onto Windows or default macOS targets.

## Phase 5 - Planning and Execution Boundary

- [x] Split logical planning from execution-time byte reads and rewrite materialization
- [x] Introduce a reusable source-entry planning boundary shared by bundle archives and normalized external packages
- [ ] Keep plan previews stable while deleting execution-only planning work
- [ ] Document which data belongs only to execution preparation and must not leak into public plan payloads
Current cleanup: public bundle apply operations no longer expose rewrite-related execution detail such as per-entry `rewrite_applied` or `rewrite_count`, and public plan summaries no longer expose `files_to_rewrite`; these now stay inside execution preparation or final execution results only.

## Phase 6 - Safety Hardening

- [x] Redesign backup restore as a safer transaction-style pipeline
- [x] Add validation before destructive replacement during restore
- [x] Replace whole-file archive buffering with streaming I/O where practical
- [ ] Add broader archive compatibility coverage
- [x] Add Windows-to-macOS migration scenario coverage
- [x] Resolve addon index local archive sources relative to the index file instead of process working directory
- [x] Harden account and character discovery using role artifacts in addition to directory layout
Current hardening: shared `addon-index.toml` files now resolve local archive inputs relative to the
index file itself, and common WTF application no longer auto-selects a discovered account just
because only one directory was found. Local account discovery now requires account-level or
character-level artifacts instead of trusting raw directory shape alone.

## Phase 7 - Integration Readiness

- [ ] Rewire CLI handlers onto reusable service or task boundaries
- [x] Introduce a shared `core::app::AppRuntime` and make addon-facing services consume injected addon-provider policy
- [x] Introduce `core::app::InstallationService` and move installation discovery onto runtime-controlled host and scan-root policy
- [ ] Turn `core::app` from forwarding facades into real service contracts with injected runtime policy
- [ ] Define desktop-facing service contracts
  Current candidates: `core::app::InstallationService`, `core::app::ExternalPackageService`, `core::app::BundleService`, `core::app::AddonLockService`, `core::app::AddonService`, `core::app::AddonIndexService`, `core::app::BackupService`
- [ ] Add progress-aware reporting hooks for future frontend integration
  Current candidate: external-package analyze, plan, and apply task wrappers with dedicated task kinds
  Current candidate: `TaskRun<T>` collection mode plus callback-based task runner helpers
  Current candidate: addon install, update, and remove task wrappers plus `core::app::AddonService` progress and callback entrypoints
  Current candidate: addon index install and update task wrappers plus `core::app::AddonIndexService` progress and callback entrypoints
  Current candidate: backup restore task wrappers plus `core::app::BackupService` progress and callback entrypoints
- [ ] Document which APIs are considered stable for GUI work
  Current candidate: external-package `summary.warning_groups` plus per-warning `category/code/source_path`
Current progress: `core::app` now exposes shared `AppRuntime` plus `with_runtime()` constructors across
service facades, and `AddonService`, `AddonIndexService`, and `AddonLockService` no longer create
default addon providers internally when the caller wants an injected runtime policy.
Current progress: `AppRuntime` now also owns default host-platform, backup-directory, and bundle-output
policy for `BundleService`, `ExternalPackageService`, and `BackupService`, so the app boundary can
materialize stable defaults before domain functions run instead of letting CLI or domain code fall
back to ambient process state.
Current progress: installation scan, inspect, and resolve now also have an app-facing
`InstallationService`, and CLI installation entrypoints no longer call `core::install` directly;
runtime can override host-platform plus candidate scan roots before install discovery reaches the
domain layer.
Current progress: `core::app::HearthSyncApp` now acts as the first stable top-level application
entrypoint, CLI handlers construct app services from it, and the old public
`core::install::{scan_installations, inspect_installation, resolve_installation}` helpers are now
crate-internal so the app boundary owns install discovery for frontend callers.
Current gate: the app boundary should not be declared stable for GUI work until stable desktop API
contracts are documented, addon and backup tasks expose more granular progress where users wait
longest, and the remaining app-facing services stop acting like thin forwarding facades.
