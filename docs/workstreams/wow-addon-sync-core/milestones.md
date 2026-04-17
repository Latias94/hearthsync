# WoW Addon Sync Core Milestones

## M0 - Core Workstream Established

### Status

Completed on 2026-04-16

### Goal

Create a dedicated architecture track for the reusable application core.

### Deliverables

- `design.md`
- `research.md`
- `todo.md`
- `milestones.md`
- `decisions.md`

### Exit Criteria

- reusable architecture ownership no longer lives only in the CLI workstream
- the first bounded refactor slice is documented

## M1 - Reusable Library Boundary

### Status

In progress

### Goal

Make the package consumable as a library before deeper internal refactors begin.

### Deliverables

- library crate entrypoint
- binary shell that depends on the library
- documented public API direction

### Exit Criteria

- CLI still builds and behaves as before
- core modules are reachable from the library boundary

## M2 - Task and Provider Contracts

### Status

In progress on 2026-04-16

### Goal

Define stable long-running operation boundaries for CLI and future desktop reuse.

### Deliverables

- task request and result conventions
- progress event model
- cancellation model
- provider abstraction boundary
- provider HTTP client port with configurable default implementation

### Exit Criteria

- bundle and addon operations can be described as reusable tasks
- provider access no longer requires direct HTTP client ownership in calling code

### Current Notes

- bundle apply and external-package apply now emit per-operation `executing` progress events instead of only one coarse execution-phase event
- cancellation during bundle and external-package execution now aborts inside the operation loop instead of waiting for the next phase boundary, and successful rollback preserves cancelled semantics instead of rewriting them as validation errors

## M3 - Import Normalization and Restore Safety

### Status

In progress on 2026-04-16

### Goal

Support real-world UI package import and strengthen destructive recovery flows.

### Deliverables

- third-party package analyzer
- normalized import model
- explicit normalized-bundle export workflow for external package reuse
- direct external package plan and apply wrapper APIs
- safer restore pipeline
- improved archive and migration coverage

### Exit Criteria

- external UI package layouts can be analyzed into product resource groups
- normalized external packages can flow through the existing bundle plan and apply pipeline
- restore semantics are explicit, validated, and rollback-aware

### Current Notes

- checked-in anonymized external-package fixtures and archive-compatibility tests now cover clean imports, dirty mixed-case packages, duplicate-normalization conflicts, invalid or directory-only archives, and unsafe zip path rejection across directory, archive, and Windows-to-macOS policy-override regression tests
- external-package analyzer warnings now expose structured category and code metadata instead of only raw strings
- external-package summaries now expose stable `warning_groups` aggregates for JSON and future GUI consumers, with CLI text output reusing the same grouped summary instead of reconstructing warning buckets ad hoc
- backup restore now validates backup metadata, declared groups, destination collisions, and target flavor before replacement, and it uses an internal transactional checkpoint so failed restores roll back to the pre-restore state instead of leaving a half-restored installation behind
- core archive file copy helpers now exist as shared infrastructure, and bundle packing, backup creation, addon archive extraction, and external-package normalization no longer buffer whole files in memory when a direct stream-to-zip or stream-to-disk path is sufficient
- the temporary first-party bundle bridge is now considered a transitional implementation detail for direct external-package flows, not the target architecture
- direct `external-package plan` and `external-package apply` now prepare from normalized source entries without temporary bundle repacking, while explicit normalized-bundle export remains available for workflows that intentionally want a reusable archive

## M3.5 - Direct External-Package Planning Path

### Status

Completed on 2026-04-17

### Goal

Remove redundant normalization and repacking from direct external-package plan and apply.

### Deliverables

- direct external-package plan path that works from normalized source entries
- direct external-package apply path that no longer requires repacking into a temporary bundle
- deleted code from the old mandatory bridge where it is no longer needed

### Exit Criteria

- `external-package plan` does not need to create a temporary first-party bundle archive
- `external-package apply` does not need to create a temporary first-party bundle archive
- explicit normalized-bundle creation still works for workflows that intentionally want a reusable archive

### Current Notes

- bundle archives and normalized external packages now share a reusable source-entry preparation boundary before execution
- bundle planning now has an internal logical-planning stage and a later execution-preparation stage, even though public plan APIs still finalize stable preview actions today
- public bundle apply operations no longer leak rewrite-related execution detail such as per-entry `rewrite_applied` or `rewrite_count`, and public plan summaries no longer leak `files_to_rewrite`; execution-only rewrite state now stays internal while final apply results still report `rewritten_files`
- direct external-package apply now reuses the same prepared-execution path as bundle apply instead of routing through a fake bundle apply entrypoint

## M4 - CLI Rewire onto Core Services

### Goal

Make the CLI a thin consumer of reusable core services and tasks.

### Deliverables

- CLI handlers aligned to service or task boundaries
- clearer separation between output rendering and domain execution

### Exit Criteria

- CLI no longer drives product architecture decisions by itself
- core contracts are the same ones future frontend code will use

### Current Notes

- `core::app::ExternalPackageService` is now the first app-facing service façade, and the CLI `external-package` handler consumes it instead of calling bundle-domain functions directly
- external-package analyze, plan, and apply now expose explicit task wrappers and dedicated task kinds for shared progress and cancellation handling
- `core::task` now also offers collected-progress and callback-based task runners, so GUI callers can consume structured progress without implementing `CancellationToken` or `TaskProgressSink` manually
- `core::app` now also exposes `BundleService` and `AddonLockService`, giving bundle apply and addon-lock apply the same direct, collected-progress, and callback-based service shape as external-package flows
- `core::app::AddonService` now also exposes addon install, update, and remove through the same direct, collected-progress, and callback-based service shape, backed by dedicated addon task kinds instead of CLI-owned progress semantics
- `core::app::AddonIndexService` now also exposes addon index install and update through the same direct, collected-progress, and callback-based service shape, with dedicated index task kinds that keep top-level progress stable while reusing addon execution underneath
- `core::app::BackupService` now exposes backup create, list, and restore, and backup restore now has a dedicated task kind plus collected-progress and callback-based entrypoints for future GUI-driven recovery flows
- the CLI bundle-apply and addon-lock handlers now consume `BundleService` and `AddonLockService`, further reducing direct CLI-to-domain coupling
- the CLI bundle-archive, bundle-addon, addon-manage, addon-index, and backup handlers now also consume `core::app` services instead of calling domain modules directly

## M5 - Desktop Integration Readiness

### Goal

Reach the point where an `egui` frontend can start without forcing another architecture reset.

### Deliverables

- stable desktop-facing service contracts
- progress-friendly task model
- documented integration assumptions

### Exit Criteria

- frontend work can begin on top of existing core contracts
- no binary-only assumptions remain on the critical path
- direct external-package and bundle planning paths no longer depend on execution-shaped repacking
