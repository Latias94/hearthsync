# WoW Addon Sync Core TODO

## Current Focus

The core workstream is no longer in bootstrap mode.
It is now the main fearless-refactor track for turning the current implementation into a reusable,
cross-platform WoW sync engine that CLI and future `egui` code can both consume.

This file tracks the remaining architecture work only.
Historical baseline work belongs in `milestones.md` and `decisions.md`.

The active refactor sequence is:

1. rebaseline the workstream documents around the real remaining architecture work
2. lock author-package default apply semantics to explicit per-group policies
3. finish the planner and execution-preparation split so public plans stay logical
4. turn `core::app` into stable service contracts instead of thin forwarding facades
5. close the remaining portability and optional-helper capability gaps on top of the cleaner core

## Refactor Rules

- delete obsolete transition code once the replacement path exists and tests pass
- prefer one canonical rule per behavior instead of CLI-only and core-only duplicates
- prefer explicit false negatives over silent destructive mis-targeting for account and character data
- keep direct external-package import on the same planning and safety model as first-party bundles

## R0 - Workstream Rebaseline

- [x] Move reusable architecture ownership into `wow-addon-sync-core`
- [x] Repoint CLI workstream notes so they stop owning core architecture
- [x] Replace the old accumulation-style TODO with a refactor-sequence view
- [x] Record the next bounded slices in `milestones.md`

## R1 - Author Package Default Semantics

Goal: direct author-package import must stop behaving like a merge-first prototype and instead
default to explicit group semantics that match real UI-package expectations.

- [x] Define one shared default profile for author-package import:
  `addons=mirror`, `wtf_common=share`, `wtf_characters=replace_selected`, `fonts=mirror`,
  `interface_assets=mirror`, `create_backup=true`
- [x] Make `external-package` manifest creation use that profile when `apply_defaults` is omitted
- [x] Make CLI partial overrides inherit the shared profile instead of falling back to `merge`
- [x] Add regression coverage for default cleanup and preserve counts on author-package plan
- [x] Add regression coverage for CLI partial-override composition
- [x] Record the semantic decision in `decisions.md`

Exit criteria:

- applying a real-world author package without policy flags no longer leaves stale addons, stale
  fonts, or stale interface assets mixed into the target installation
- changing one CLI policy flag does not silently reset every other group back to `merge`

## R2 - Logical Planning Boundary

Goal: public planning APIs should describe intent, not execution staging internals.

- [x] separate logical planning DTOs from execution-preparation data that only exists to support
  byte reads, rewrites, and cleanup materialization
  Completed: the planner no longer depends on an execution-only `source_for_entry` callback;
  external-package normalized entry source resolution stays under the prepared apply source used by
  execution.
  Completed: planner internals now split “operations already logically determined” from
  “entries that still need existing-target preview finalize”, and preview finalize resolves into a
  preview-only operation model instead of reusing `PreparedApplyOperation`.
  Completed: public `bundle plan` and `external-package plan` no longer route through
  `PreparedBundleApply`; apply paths project the resolved preview into a lean execution payload only
  at the final boundary.
- [x] reduce public-plan dependence on entry-byte reads where no rewrite or content comparison is
  required for the preview contract
  Completed: planner skips source-byte reads for deterministic `Add` operations, and actual rewrite
  application is decided during execution instead of being precomputed during plan preparation.
  - [x] make the direct external-package path and first-party bundle path share the same logical
    planner boundary instead of only the same execution-preparation boundary
    Completed: the remaining raw planner byte-reader seam is now test-only; non-test callers stay on
    prepared apply sources or the `core::app` boundary instead of depending on closure-driven planner
    helpers.
- [x] document which data is allowed in public plan payloads and which data must remain execution-only
  Completed: `design.md` and `decisions.md` now explicitly limit public plan payloads to logical
  preview data and forbid rewrite vectors, source maps, staging paths, and other execution-only
  state from leaking into `bundle plan` or `external-package plan`.
- [x] delete the remaining execution-shaped planning helpers once the smaller logical path is in place
  Completed: resolved preview now owns the public `BundleApplyPlan` directly, and `plan` / `prepare`
  consume the same resolved result instead of routing through separate projection helpers for plan
  and execution preparation.

Exit criteria:

- `bundle plan` and `external-package plan` remain stable dry-run APIs for CLI and future GUI use
- execution-only rewrite/materialization state does not leak into public plan contracts

## R3 - `core::app` Contract Stabilization

Goal: the reusable app boundary should own runtime policy, requests, results, and task behavior in
ways that a future frontend can depend on without learning internal domain seams.

- [x] define which `core::app` services are intended to be GUI-stable first
  Completed: the first-wave GUI-stable set is now explicitly defined as installation, addon,
  bundle, external-package, and backup services; addon-index and addon-lock remain available app
  services but are not part of the first-wave stability promise yet.
- [x] keep app request and result types app-owned where frontends depend on them directly
  Completed: `InstallationService::resolve` now returns one shared app-owned resolved
  installation value, and app requests that target an installation now consume that value instead
  of leaking domain `DetectedFlavorInstallation` directly through the frontend boundary. Bundle
  and external-package requests now also use shared app-owned apply strategy values for
  target-account mappings and author-package default-policy overrides, so frontends no longer need
  domain `BundleApplyMappings` or `ApplyDefaults` just to drive stable bundle/external-package
  flows. `AddonService` install/list contracts now also use one shared app-owned addon package
  metadata value instead of exposing domain `AddonPackageMetadata` through stable addon requests or
  tracked-package results. Stable pack/apply/export results now also share one app-owned manifest
  value tree, so `BundleService` and `ExternalPackageService` no longer require domain
  `BundleManifest` at the stable app boundary just to accept a manifest request or return a full
  manifest payload. Stable installation selection, bundle source metadata, external-package source
  metadata, and runtime host defaults now also share app-owned `HostPlatformValue` and
  `WowFlavorValue`, while manifest mapping rules and installation-health payloads now also use
  app-owned `CharacterMappingModeValue` and `HealthStatusValue`. Frontend callers no longer need
  domain install or manifest enums just to express host policy, selected flavor, manifest mapping
  rules, or installation-health state. The same response-boundary cleanup now covers the full
  current app service surface too: installation, addon, backup, bundle, external-package,
  addon-index, addon-lock, and bundle-addon-lock app responses use crate-internal `from_domain`
  factories instead of public `From<domain>` impls, so stable result types no longer advertise
  those domain conversions as part of the frontend-facing trait surface.
- [ ] remove remaining thin-forwarder service behavior by moving real normalization and policy
  ownership into the app boundary
  Current progress: runtime-backed default injection for backup directories, bundle output
  directories, and author-package source platform now lives on app request contracts via
  `apply_runtime_defaults` instead of being split across private service-local `normalize_*`
  helpers. This closes a real behavior gap too: addon, addon-index, and addon-lock mutation
  services now honor the shared runtime default backup directory instead of only bundle, backup,
  and external-package flows doing so. `HearthSyncApp` now also exposes direct frontend entrypoints
  for installation, addon, bundle, external-package, backup, addon-index, and addon-lock direct
  operations, so CLI no longer needs to compose service selection and installation resolution
    manually around the app boundary. The same frontend root now also forwards the stable long-
    running task entrypoints for addon, backup restore, bundle apply, and external-package flows, so
    future GUI work does not need to drop back to raw services just to collect progress or stream
    callbacks. `StableAppServices` now mirrors that first-wave stable direct/task surface too, so the
    smaller GUI-stable boundary is an explicit code contract instead of a service-factory-only hint.
    Current cleanup: `HearthSyncApp` now delegates those first-wave stable direct/task entrypoints
    through `StableAppServices`, leaving one stable implementation path for future GUI-facing
    operations while keeping addon-index/addon-lock access available from the full app root.
    Current cleanup: raw `StableAppServices` service accessors and direct runtime access are now
    crate-visible only, so external callers stay on stable direct/task entrypoints instead of
    treating the stable boundary as another service factory.
    Current cleanup: `HearthSyncApp` now composes and dereferences the stable boundary instead of
    repeating first-wave direct/task wrappers, so the full app root only adds non-stable addon
    index / addon lock entrypoints on top of one shared GUI-facing surface.
    Current cleanup: raw `runtime()` access on individual app services is now test-only, so app
    runtime wiring stays an internal assembly concern rather than another public extension seam.
    Current cleanup: internal `*Service` implementations are no longer publicly re-exported from
    `core::app`, so external callers naturally converge on `HearthSyncApp` / `StableAppServices`
    instead of bypassing the intended app-owned boundary.
    Current cleanup: internal service convenience constructors now stay test-only as well, so the
    remaining production-facing entrypoints are the app roots rather than implementation helpers.
    Current cleanup: request-side `apply_runtime_defaults()` helpers are now crate-visible only, so
    runtime default projection stays inside app assembly instead of leaking onto the public request
    API surface.
    Current cleanup: app response DTOs no longer own CLI text rendering or redundant accessor sugar;
    presentation formatting now lives in CLI code, and wrapper results behave like data objects
    instead of mini service facades.
    Current cleanup: display-oriented helper methods on app value types are moving back to CLI or
    runtime edges, so public request/response enums remain data shapes instead of mixed-in
    formatting utilities.
    Current cleanup: app-owned contract modules are now also split by domain, with
    `types/{install,addon,bundle,backup,runtime,external_package}` and
    `response/{installation,addon,addon_index,addon_lock,backup,bundle,external_package}`
    replacing the previous monolithic files. This keeps the stable app boundary easier to review,
    evolve, and bind from a future `egui` frontend.
    Current cleanup: request contracts now follow the same domain split under
    `request/{installation,addon,addon_index,addon_lock,backup,bundle,external_package}`, and the
    remaining external-package `apply_runtime_defaults()` helpers are crate-visible again rather
    than public API. Runtime default projection stays inside app assembly.
    Current cleanup: app request contracts no longer expose public `From<app request> for domain`
    conversions. Crate-internal projection now lives on explicit `into_domain_*` helpers so the
    stable frontend boundary no longer advertises domain request types as part of its public trait
    surface.
    Installation scan/inspect/resolve host policy is now also owned by runtime or request-side app
    helpers instead of being reassembled inside `InstallationService`, and the remaining thin
    installation-targeted read/plan projections now sit on app request contracts instead of
    service-local `let installation = request.installation.into()` glue. Runtime-backed mutation
    requests now also own the full "apply defaults, then project into the domain request" step, so
    services no longer coordinate `request.apply_runtime_defaults(&self.runtime).into()` by hand
    across addon, addon-index, addon-lock, backup, bundle, and external-package flows. The
    remaining work in this area is narrower and mostly about any still-meaningful behavioral policy
    that services own, rather than scattered path/default patching, host injection, or root-level
    orchestration gaps.
- [x] document stable progress expectations for long-running bundle, external-package, addon, and
  backup tasks
  Completed: `core::app` task entrypoints now have one documented wrapper contract. Direct calls
  run with no-op progress and no cancellation, collecting-progress calls return `TaskRun<TResult>`
  with ordered `TaskProgressEvent` payloads, and callback-based calls stream the same event shape
  while honoring caller-supplied cancellation checks. Successful long-running tasks begin with
  `Preparing`, end with `Completed`, and may report task-specific intermediate phases such as
  `Planning`, `BackingUp`, `Executing`, or `Verifying`. Those stable task-contract types are now
  also surfaced from `core::app`, so frontend callers do not need to import `core::task`
  separately just to consume app-service progress behavior.
- [x] keep optional provider/helper capability switches behind runtime/service boundaries instead of
  leaking them into CLI orchestration
  Completed: `AppRuntime` no longer requires addon-provider domain option types at the frontend
  boundary. Default provider cache/retry configuration now uses app-owned
  `AddonProviderOptionsValue` and `AddonProviderRetryPolicyValue`, custom provider injection is
  crate-internal, and helper strategy lives on runtime instead of bundle-domain plan DTOs.
  `AppRuntime`, `HearthSyncApp`, and `StableAppServices` now expose one app-owned
  `AppRuntimeCapabilitiesValue` snapshot so frontend callers can read provider/helper capability
  state without inferring it from ad hoc `Option` semantics or planner details. Any future
  non-native helper capability should extend that runtime-owned contract rather than reappearing as
  an ambient planner concern.

Exit criteria:

- `core::app::HearthSyncApp` is a credible frontend root instead of only a service factory
- CLI and future `egui` code can depend on the same task and service contracts

## R4 - Portability and Capability Hardening

Goal: finish the remaining cross-platform and helper-boundary gaps after the architecture is clean
enough that these rules live in one place.

- [x] define the optional external-helper capability boundary explicitly
  Completed: runtime capability reporting now distinguishes external-helper policy and
  availability from the currently active `helper_strategy`. `AppRuntimeCapabilitiesValue`
  exposes an explicit `external_helper` snapshot, while plan/apply result payloads continue to
  report the actual active strategy. This keeps external helpers optional accelerators instead of
  turning them into ambient planner or service assumptions before any concrete helper backend
  exists.
- [ ] broaden archive-compatibility coverage for author packages and large real-world inputs
- [ ] verify the cleaned-up contracts against more Windows-to-macOS author-package scenarios
- [ ] tighten any remaining path portability edge cases around case folding, archive metadata, and
  caller-working-directory assumptions
  Current progress: bundle export no longer defaults output paths or relative output references
  against the ambient process working directory, and relative bundle addon-index references now
  require an explicit `manifest_base_dir` instead of silently resolving against `cwd`. Remaining
  work is mainly broader archive-metadata hardening plus any other case-folding or ambient-path
  edges outside the bundle export path.

Exit criteria:

- Windows and macOS callers share one deterministic import contract
- helper-assisted paths, if added later, remain optional accelerators rather than architecture owners
