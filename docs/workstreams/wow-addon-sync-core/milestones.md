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

### Current Notes

- `core::app::HearthSyncApp` is now the first explicit top-level API candidate for reusable
  consumers; it owns one shared `AppRuntime` and produces the installation, addon, bundle,
  external-package, backup, addon-index, and addon-lock services from that runtime
- the first smoke coverage now asserts that `HearthSyncApp` hands the same runtime policy to all
  app-facing services

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
- addon install, addon index install, update, and lock-apply preparation now propagate task
  cancellation into provider-backed archive downloads, the default reqwest client now has explicit
  connect and request timeouts, and cancelled downloads stop without being retried
- addon install, addon update, addon remove, addon index install, addon index update, and backup
  restore now also emit detail-level `executing` progress from inside their real filesystem loops,
  and those loops re-check cancellation instead of only trusting outer phase boundaries
- addon lock apply now also reuses those task-aware addon mutation executors, so remove, update,
  install, and metadata-only lock actions surface execution-detail progress under one stable top-level
  task kind instead of disappearing behind one coarse `executing` event

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

## M3.6 - Real-World Import Compatibility Hardening

### Status

In progress on 2026-04-17

### Goal

Close the gap between current normalized-import assumptions and real addon/WTF layouts while
deleting duplicated compatibility code.

### Deliverables

- shared addon-root classifier reused by addon archive install and external-package analysis
- tolerant addon-root detection for archives whose `.toc` file name differs from the directory name
- deleted duplicate WTF scope and addon-root classification helpers where one shared rule is enough
- documented and then implemented support for root-level `WTF/Account/SavedVariables`

### Exit Criteria

- a real-world addon archive is not rejected only because its `.toc` file name does not exactly
  match the directory name
- external-package analysis and addon archive install do not maintain separate addon-root
  heuristics
- root-level `WTF/Account/SavedVariables` is no longer silently dropped as unsupported import data

### Current Notes

- this is a bounded refactor slice inside the existing core workstream, not a new parallel
  workstream
- the first sub-slice now shares addon-root detection between addon archive install and
  external-package analysis, accepts variant `.toc` naming, and deletes duplicate WTF scope
  classification code before the broader WTF normalization gap is addressed
- the second sub-slice now normalizes root-level `WTF/Account/SavedVariables` into the common
  WTF model across first-party bundles, external-package analysis, planning, apply cleanup, and
  Lua rewrite targeting, so this data is imported instead of being downgraded to warnings
- the portability sub-slice now rejects normalized path sets that differ only by case when they
  would collide on Windows or default macOS targets, and addon-index relative local archive sources
  now resolve against the index file instead of ambient working directory
- the account-targeting sub-slice now requires account-level or character-level artifacts for local
  account discovery, and common WTF application no longer auto-selects a target account only
  because one discovered directory happened to exist

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
- `core::app` now also has a shared `AppRuntime` boundary with `with_runtime()` constructors on every service, and addon-related services no longer hide a hard-coded default addon provider behind their app-facing methods; callers can inject provider policy for search, install, index update, and lock apply flows
- `AppRuntime` now also carries default host-platform, backup-directory, and bundle-output policy, and `BundleService`, `ExternalPackageService`, and `BackupService` normalize those defaults into explicit requests before crossing into bundle, external-package, or backup domain code
- the same runtime boundary now also exposes shared default-injection helpers for backup paths,
  bundle output paths, and source-platform defaults, so app services stop re-encoding the same
  missing-value policy locally
- app-facing inspect, resolve, list, and planning APIs now also use explicit owned request
  contracts instead of mixing borrowed paths, optional references, and positional parameters across
  services
- app-facing mutation APIs for addon, addon index, addon lock apply, backup, bundle, and
  external-package flows now also use explicit owned request contracts, and CLI handlers no longer
  construct domain mutation requests directly when crossing into `core::app`
- read-only app outputs for installation scan and inspect, addon inventory, addon-index inspect,
  addon-lock inspect, backup catalog list, and bundle inspect now also flow through app-owned DTOs
  instead of returning raw domain aggregate structs directly
- addon-related app outputs now also wrap source references in app-owned DTOs instead of leaking
  raw domain `AddonSourceRef` values through search, inventory, install, index, or addon-lock
  payloads
- `core::app` now also has a small shared app-type layer for reusable frontend-facing value enums,
  starting with backup groups so backup requests and results no longer expose raw domain
  `BackupGroup` values
- `core::app::InstallationService` now exposes scan, inspect, and resolve, `AppRuntime` can override installation scan roots plus host-platform policy, and CLI installation entrypoints now reach install discovery through the app service layer instead of calling `core::install` directly
- CLI handlers that need multiple capabilities now construct them from `core::app::HearthSyncApp`
  instead of instantiating unrelated service facades independently, and direct install discovery
  helpers no longer remain part of the public library hot path
- app-facing task services now also share one internal wrapper contract for direct, collected-progress,
  and callback-based execution, reducing façade duplication and making the non-GUI and future GUI paths
  reuse the same cancellation and progress wiring

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

### Current Notes

- `core::app::HearthSyncApp` remains the intended frontend root, and the earlier portability plus
  provider-cancellation correctness blockers are now addressed; the remaining desktop-readiness work
  is to stabilize explicit service contracts, improve long-running task reporting, and document what
  GUI callers may rely on as stable
- request/result ownership is now largely explicit at the app boundary; the next contract cleanup
  should focus on documenting stability, pruning remaining thin façade behavior, and deciding which
  shared value types still need app-owned wrappers before `egui` depends on them long term
