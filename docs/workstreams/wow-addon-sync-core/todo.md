# WoW Addon Sync Core TODO

## Current Focus

The current priority is to turn the existing implementation into a reusable application core before more surface area is added.

The active sequence is:

1. expose a reusable library entrypoint
2. move architecture ownership to this workstream
3. remove the temporary-bundle bridge from direct external-package plan and apply [completed on 2026-04-17]
4. make bundle planning a cleaner logical boundary
5. harden provider, task, and account-discovery contracts for future GUI reuse

## Phase 0 - Workstream Bootstrap

- [x] Create the core workstream
- [x] Record reusable core architecture goals
- [x] Record research summary and current gaps
- [x] Repoint CLI workstream notes to the new core source of truth

## Phase 1 - Library Boundary

- [x] Expose a package library entrypoint
- [ ] Decide the first stable top-level public API surface
- [ ] Add library-level smoke tests for reusable entrypoints
- [ ] Reduce binary-specific assumptions in module ownership

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

## Phase 4 - Import Normalization

- [x] Design analyzer input model for external archives and folders
- [x] Detect addon, WTF, fonts, and interface asset groups from third-party packages
- [x] Produce a normalized import model or temporary internal manifest
- [x] Reuse the same plan and execution pipeline as first-party bundles
- [x] Remove the mandatory temporary first-party bundle bridge from direct external-package plan
- [x] Remove the mandatory temporary first-party bundle bridge from direct external-package apply
- [x] Keep explicit normalized-bundle export as an optional workflow, not an internal dependency

## Phase 5 - Planning and Execution Boundary

- [x] Split logical planning from execution-time byte reads and rewrite materialization
- [x] Introduce a reusable source-entry planning boundary shared by bundle archives and normalized external packages
- [ ] Keep plan previews stable while deleting execution-only planning work
- [ ] Document which data belongs only to execution preparation and must not leak into public plan payloads

## Phase 6 - Safety Hardening

- [x] Redesign backup restore as a safer transaction-style pipeline
- [x] Add validation before destructive replacement during restore
- [x] Replace whole-file archive buffering with streaming I/O where practical
- [ ] Add broader archive compatibility coverage
- [x] Add Windows-to-macOS migration scenario coverage
- [ ] Harden account and character discovery using role artifacts in addition to directory layout

## Phase 7 - Integration Readiness

- [ ] Rewire CLI handlers onto reusable service or task boundaries
- [ ] Turn `core::app` from forwarding facades into real service contracts with injected runtime policy
- [ ] Define desktop-facing service contracts
  Current candidates: `core::app::ExternalPackageService`, `core::app::BundleService`, `core::app::AddonLockService`, `core::app::AddonService`, `core::app::AddonIndexService`, `core::app::BackupService`
- [ ] Add progress-aware reporting hooks for future frontend integration
  Current candidate: external-package analyze, plan, and apply task wrappers with dedicated task kinds
  Current candidate: `TaskRun<T>` collection mode plus callback-based task runner helpers
  Current candidate: addon install, update, and remove task wrappers plus `core::app::AddonService` progress and callback entrypoints
  Current candidate: addon index install and update task wrappers plus `core::app::AddonIndexService` progress and callback entrypoints
  Current candidate: backup restore task wrappers plus `core::app::BackupService` progress and callback entrypoints
- [ ] Document which APIs are considered stable for GUI work
  Current candidate: external-package `summary.warning_groups` plus per-warning `category/code/source_path`
