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

- `core::app::StableAppServices` is the intended stable frontend root
- CLI and future `egui` code can consume the same services and task contracts
- the app boundary owns defaulting and policy decisions that should not leak into command handlers

### Current Notes

- the project already has the right direction: `StableAppServices`, `ExtendedAppServices`,
  `AppRuntime`, and app-owned DTOs
- the remaining work is about contract ownership and stability, not about inventing another façade layer
- the first-wave GUI-stable service set is now explicit: installation, addon, bundle,
  external-package, and backup services
- `ExtendedAppServices` now composes a dedicated stable-service boundary so future frontend work does
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
  `ExtendedAppServices`, or `StableAppServices` without reading planner details or inferring custom
  provider injection from optional fields
- bundle and external-package app responses now build from crate-internal domain projection helpers
  instead of public `From<domain>` trait impls, reducing the stable app boundary's visible domain
  coupling
- installation, addon, backup, addon-index, addon-lock, and bundle-addon-lock app responses now
  follow the same rule too, so the remaining response boundary no longer advertises public domain
  conversion traits for these main response payloads either
- `StableAppServices` now exposes the first-wave stable direct/task entrypoints, so the explicit
  GUI-stable boundary is a real stable frontend contract instead of only a named service container
- `ExtendedAppServices` now focuses on the less-stable addon-index, addon-lock, and bundle-addon-lock
  operations while composing `StableAppServices` explicitly for shared runtime policy
- stable CLI handlers now construct `StableAppServices` directly for installation/addon/backup/
  bundle/external-package flows, leaving `ExtendedAppServices` focused on the less-stable addon-index,
  addon-lock, and bundle-addon-lock entrypoints that still sit outside the first stable wave
- CLI command handlers now share one `cli::app_support` entry helper for service construction and
  installation resolution, reducing duplicate app-boundary glue around both stable and extension
  flows
- raw `StableAppServices` service accessors and direct runtime access are now crate-visible only, so
  the public stable boundary stays centered on direct/task entrypoints instead of leaking a second
  service-factory-style API
- `ExtendedAppServices` now composes `StableAppServices` through an explicit stable bridge instead of
  `Deref` compatibility, so the app root no longer masquerades as the stable surface by accident
- the broader non-stable app root is now named `ExtendedAppServices`, which makes its role as an
  extension boundary clearer than the previous `HearthSyncApp` name
- raw `runtime()` access on individual app services is now test-only, so runtime wiring is kept as
  an internal assembly detail instead of a public extension seam
- internal `*Service` implementation types are now crate-only re-exports, so public consumers are
  steered toward `ExtendedAppServices` / `StableAppServices` instead of depending on internal app wiring
- internal service convenience constructors are now test-only, so production assembly routes
  through the explicit app roots instead of scattered implementation helpers
- request-side `apply_runtime_defaults()` helpers are now crate-only, so runtime default projection
  remains an internal app assembly concern instead of part of the public request API surface
- app response DTOs no longer own CLI text rendering or redundant accessor sugar, so response
  shapes stay closer to transport data while presentation and wrapper ergonomics live at the edges
- display-oriented helper methods on app value types are moving back to CLI or runtime edges, so
  public app enums remain contract data instead of accumulating formatting utilities
- app-owned contract modules are now also split by domain, with smaller `types/*` and `response/*`
  files replacing the old monolithic contract modules; this reduces review friction now and keeps
  future `egui` binding work from depending on one oversized app-contract file
- app request contracts now follow the same `request/*` domain split, and the remaining
  external-package runtime-default helpers are crate-visible rather than public API, keeping
  runtime projection as internal app assembly behavior
- app request contracts no longer rely on public `From<app request> for domain request` trait
  impls; domain projection now stays on crate-internal helper methods, matching the response-side
  boundary cleanup and reducing visible frontend coupling to domain request types
- app value contracts now also use crate-internal `from_domain()` / `into_domain()` helpers instead
  of public domain conversion trait impls, so frontend-facing value shapes no longer advertise
  domain types as part of their stable contract surface
- app flavor values no longer expose folder-name layout helpers publicly; folder-name rules stay
  owned by the install domain, while CLI-only display slugs remain crate-internal
- large `core::app` modules now keep regression tests in sibling `*/tests.rs` files, so
  production contract/service code is easier to review without weakening app-layer coverage
- runtime default/path projection helpers are crate-visible only again, and `ExtendedAppServices`
  now exposes an explicit stable bridge instead of `Deref<Target = StableAppServices>`
- the remaining raw planner byte-reader seam is now test-only, so future `egui` integration can
  treat `ExtendedAppServices` / `StableAppServices` as the intended stable boundary instead of depending
  on internal planning helpers
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
- bundle export no longer depends on ambient `cwd` for default output paths, relative output
  references, or relative addon-index metadata resolution; those flows now use explicit base-dir
  rules instead of process-global state
- the next `M4` slices should now focus on archive compatibility, Windows-to-macOS regression
  coverage, and remaining path-portability hardening
