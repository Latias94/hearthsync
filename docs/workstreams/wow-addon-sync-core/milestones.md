# WoW Addon Sync Core Milestones

## M0 - Core Architecture Ownership Established

### Status

Completed on 2026-04-18

### Goal

Make `wow-addon-sync-core` the architecture source of truth for the reusable product core.

### Deliverables

- `design.md`
- `todo.md`
- `milestones.md`
- `decisions.md`
- CLI workstream references that point back to the core workstream for shared architecture

### Exit Criteria

- reusable architecture decisions no longer live primarily in CLI notes
- the next bounded fearless-refactor slices are documented in one place

## M1 - Author Package Default Semantics

### Status

Completed on 2026-04-18

### Goal

Make direct author-package import default to explicit resource-group semantics instead of the old
merge-first prototype behavior.

### Deliverables

- one shared author-package default apply profile
- core manifest creation that uses that profile when `apply_defaults` is omitted
- CLI override composition that inherits the shared profile for unspecified groups
- regression tests for default plan behavior and partial CLI overrides

### Exit Criteria

- direct `external-package plan/apply` without policy flags mirrors addons, fonts, and interface
  assets by default
- common WTF defaults to `share`
- character WTF defaults to `replace_selected`
- overriding one CLI policy flag does not silently reset other groups back to `merge`

### Current Notes

- this milestone intentionally changes product behavior because the previous default semantics were
  not defensible for real-world author UI packages
- backup creation remains enabled by default on the new profile

## M2 - Logical Planner Boundary

### Status

Completed on 2026-04-18

### Goal

Finish the split between logical planning and execution preparation so public plan APIs stay stable
and preview-friendly.

### Deliverables

- a smaller logical planner boundary shared by bundle archives and normalized external packages
- execution-only preparation kept behind internal helpers
- documented rules for what may appear in public plan payloads
- deleted transition helpers that only exist to preserve execution-shaped planning

### Exit Criteria

- `bundle plan` and `external-package plan` are logical previews, not staging pipelines in disguise
- execution-only rewrite/materialization details remain internal
- future GUI dry-run views do not need to understand execution-preparation artifacts

### Current Notes

- the mandatory temporary bundle-repack bridge is already gone from direct external-package plan/apply
- the remaining work is to keep deleting execution-shaped preview work from the planner internals
- the planner no longer needs an execution-only `source_for_entry` callback just to support direct
  external-package apply; normalized entry-to-source resolution now lives with the prepared apply
  source instead of being threaded through logical planning
- planner internals now explicitly split “logical operations already known without byte reads” from
  “existing-target entries that still need preview finalize”
- the second phase now resolves preview-only operations instead of reusing
  `PreparedApplyOperation`, making the preview-finalize boundary explicit without leaking execution
  payloads back into the planner
- planner preview no longer reads source bytes for deterministic `Add` operations, and rewrite
  applicability is now finalized during execution rather than being stored on prepared plan operations
- public `bundle plan` and `external-package plan` now resolve preview plans directly instead of
  routing through `PreparedBundleApply` or the external-package apply-preparation path
- `PreparedApplyOperation` is now a lean execution payload carrying only action, source identity,
  destination, and rewrites; plan-only metadata stays on preview/public plan operations
- the workstream docs now explicitly define the public-plan contract: logical preview data is
  allowed, while rewrite vectors, source maps, staging paths, and rollback bookkeeping remain
  execution-only
- resolved preview now owns the public plan directly, so plan and apply-preparation consume one
  shared resolved boundary instead of maintaining separate projection helpers after preview finalize

## M3 - Stable App Contracts for Frontends

### Status

Active

### Goal

Turn `core::app` from a useful façade layer into a stable application boundary for CLI and future
desktop work.

### Deliverables

- documented GUI-stable service set
- stable app-owned request and result contracts where frontends depend on them
- shared progress and cancellation expectations for long-running operations
- runtime-owned policy injection for provider and helper capabilities

### Exit Criteria

- `core::app::HearthSyncApp` is the intended frontend root
- CLI and future `egui` code can consume the same services and task contracts
- the app boundary owns defaulting and policy decisions that should not leak into command handlers

### Current Notes

- the project already has the right direction: `HearthSyncApp`, `AppRuntime`, and app-owned DTOs
- the remaining work is about contract ownership and stability, not about inventing another façade layer
- the first-wave GUI-stable service set is now explicit: installation, addon, bundle,
  external-package, and backup services
- `HearthSyncApp` now also exposes a dedicated stable-service boundary so future frontend work does
  not need to treat addon-index and addon-lock as equally stable day-one contracts
- the first shared app-owned input value is now explicit too: resolved installations flow through
  one reusable app value object instead of leaking domain `DetectedFlavorInstallation` through app
  requests
- bundle and external-package stable requests now also share app-owned apply-strategy value
  objects, so mapping input and author-package policy overrides no longer require frontend callers
  to construct domain `BundleApplyMappings` or `ApplyDefaults`
- addon install/list stable contracts now also share one app-owned addon package metadata value, so
  stable addon callers no longer depend on domain `AddonPackageMetadata` for request metadata or
  tracked-package metadata results
- stable bundle/external-package manifest payloads now share one app-owned manifest value tree, so
  stable callers no longer depend on domain `BundleManifest` for pack requests or full manifest
  result payloads
- stable install/runtime/bundle-source contracts now also share app-owned `HostPlatformValue` and
  `WowFlavorValue`, so frontend callers no longer depend on domain install enums for host defaults,
  selected flavor input, resolved installations, or source compatibility metadata
- stable manifest mapping rules and installation-health payloads now also use app-owned
  `CharacterMappingModeValue` and `HealthStatusValue`, closing the remaining small enum leaks on
  the frontend-facing app boundary
- long-running addon, bundle, external-package, addon-index, addon-lock, and backup app tasks now
  share one documented progress contract across direct, collected-progress, and callback entrypoints
- runtime-backed backup/output/source-platform default injection now lives on shared app request
  contracts, so addon, addon-index, addon-lock, backup, bundle, and external-package services no
  longer each carry fragmented service-local normalization for those defaults
- installation scan/inspect/resolve policy now also lives on runtime or request-side app helpers,
  so `InstallationService` no longer decides host injection or scan-root branching itself
- the remaining thin installation-targeted read/plan conversions now sit on app request helpers,
  so addon list, addon-lock read/plan flows, and bundle plan helpers no longer keep
  `request.installation.into()` glue in service bodies
- runtime-backed mutation requests now also own the final normalized domain projection, so addon,
  addon-index, addon-lock, backup, bundle, and external-package services no longer coordinate
  `apply_runtime_defaults(...).into()` as a separate service-local step
- default addon-provider cache and retry configuration now also uses app-owned runtime values
  instead of leaking provider-domain option structs through `core::app::AppRuntime`
- custom addon-provider injection is now crate-internal runtime composition, so the public app
  boundary no longer exposes provider trait seams just to support tests
- stable task contract types are now surfaced from `core::app`, so app-service callers no longer
  need to import `core::task` directly for progress collection or callback streaming
- helper strategy has now been removed from bundle-domain plan DTOs and is reported from
  `core::app::AppRuntime` instead, so optional-helper capability state no longer leaks out of the
  planner boundary
- runtime capability state is now also exposed through one app-owned `AppRuntimeCapabilitiesValue`
  snapshot, so frontend callers can inspect provider/helper mode from `AppRuntime`,
  `HearthSyncApp`, or `StableAppServices` without reading planner details or inferring custom
  provider injection from optional fields
- bundle and external-package app responses now build from crate-internal domain projection helpers
  instead of public `From<domain>` trait impls, reducing the stable app boundary's visible domain
  coupling
- installation, addon, backup, addon-index, addon-lock, and bundle-addon-lock app responses now
  follow the same rule too, so the remaining response boundary no longer advertises public domain
  conversion traits for these main response payloads either
- `HearthSyncApp` now also exposes direct app-operation entrypoints for the flows the CLI actually
  drives, so callers can stay on one frontend root instead of stitching together installation
  resolution and per-service dispatch by hand
- the same frontend root now forwards stable long-running task entrypoints too, so addon, backup
  restore, bundle apply, and external-package progress/callback flows no longer require callers to
  drop down to raw service accessors just to stay on the `core::app` boundary
- `StableAppServices` now exposes the same first-wave stable direct/task entrypoints, so the
  explicit GUI-stable boundary is no longer just a named service container but a real stable
  frontend contract
- the remaining `M3` work is now primarily behavioral: thin-forwarder normalization or policy logic
  that still lives in app service wrappers

## M4 - Portability and Optional-Helper Hardening

### Status

Active

### Goal

Close the remaining cross-platform and optional-capability gaps on top of the cleaner architecture.

### Deliverables

- explicit external-helper capability boundary
- broader archive-compatibility coverage
- more Windows-to-macOS import regression coverage
- final cleanup of portability rules that still depend on ambient process state or case-sensitive assumptions

### Exit Criteria

- Windows and macOS callers share one deterministic author-package import contract
- helper-assisted paths remain optional accelerators instead of becoming architecture owners

### Current Notes

- `AppRuntimeCapabilitiesValue` now distinguishes external-helper policy and availability from the
  active `helper_strategy`, so future helper backends can be added as optional accelerators without
  overloading one runtime field with both desired policy and actual execution state
- public bundle and external-package plan/apply payloads still report the active
  `helper_strategy`, which remains `NativeRust` until a real helper backend exists
- the next `M4` slices should now focus on archive compatibility, Windows-to-macOS regression
  coverage, and remaining path-portability hardening
