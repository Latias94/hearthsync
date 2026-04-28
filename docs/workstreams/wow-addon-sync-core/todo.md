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
6. close remote-source freshness, transaction-complete mutation semantics, and the remaining
   high-cost preview behavior before frontend reuse depends on them
7. finish structured task progress so future frontend work can consume machine-readable task
   streams without parsing CLI text
8. keep expanding transfer-heavy progress coverage by reusing the stable task event shape instead
   of adding frontend-only progress channels
9. close the 2026-04-28 architecture-hardening findings before `egui` depends on current addon
   mutation, index update, config, provider, and task boundaries

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
- [x] remove remaining thin-forwarder service behavior by moving real normalization and policy
  ownership into the app boundary
  Completed: runtime-backed default injection for backup directories, bundle output
  directories, and author-package source platform now lives on app request contracts via
  `apply_runtime_defaults` instead of being split across private service-local `normalize_*`
  helpers. This closes a real behavior gap too: addon, addon-index, and addon-lock mutation
  services now honor the shared runtime default backup directory instead of only bundle, backup,
  and external-package flows doing so. `StableAppServices` now owns the first-wave stable direct/
    task surface for installation, addon, bundle, external-package, and backup flows, so future GUI
    work has one explicit stable app contract for both direct results and long-running task
    behavior. `ExtendedAppServices` remains available as the broader extension root for addon-index,
    addon-lock, and bundle-addon-lock operations that are still outside that stable wave.
    Current cleanup: `ExtendedAppServices` now composes the stable boundary explicitly instead of
    inheriting it through implicit `Deref` compatibility, which keeps stable and non-stable app
    promises visible at call sites.
    Current cleanup: the broader non-stable app root is now named `ExtendedAppServices`, so
    callers no longer infer from the old `HearthSyncApp` name that it is the default stable
    frontend entrypoint.
    Current cleanup: stable CLI handlers now construct `StableAppServices` directly for
    installation/addon/backup/bundle/external-package flows, while `ExtendedAppServices` remains the
    fuller root only for less-stable addon-index/addon-lock/bundle-addon-lock operations.
    Current cleanup: CLI service construction and installation-target resolution now share one
    `cli::app_support` helper, so command handlers no longer duplicate `ResolveInstallationRequest`
    assembly or drift on which app boundary they should enter through.
    Current cleanup: addon-lock CLI output now shares formatter helpers under `cli::output`, so
    repeated diff/verify/apply package rendering stays at the presentation edge instead of being
    copied across command handlers.
    Current cleanup: raw `StableAppServices` service accessors and direct runtime access now stay
    inside the `core::app` module boundary, so external callers stay on stable direct/task
    entrypoints instead of treating the stable boundary as another service factory.
    Current cleanup: the full app root now reaches the stable boundary through an explicit
    `stable()` bridge, so addon index / addon lock behavior no longer inherits stable installation
    and bundle contracts implicitly.
    Current cleanup: raw `runtime()` access on individual app services is now test-only, so app
    runtime wiring stays an internal assembly concern rather than another public extension seam.
    Current cleanup: internal `*Service` implementations are no longer re-exported outside
    `core::app`, so callers naturally converge on `ExtendedAppServices` / `StableAppServices`
    instead of bypassing the intended app-owned boundary.
    Current cleanup: internal service convenience constructors now stay test-only as well, so the
    remaining production-facing entrypoints are the app roots rather than implementation helpers.
    Current cleanup: request-side `apply_runtime_defaults()` helpers are now crate-visible only, so
    runtime default projection stays inside app assembly instead of leaking onto the public request
    API surface.
    Current cleanup: app response DTOs no longer own CLI text rendering or redundant accessor sugar;
    presentation formatting now lives in CLI code, and wrapper results behave like data objects
    instead of mini service facades.
    Current cleanup: `ExternalPackageBundleHandle` no longer relies on `Deref` field forwarding;
    callers now opt into the wrapped result explicitly through `AsRef`, keeping the stable app
    boundary honest about where the temporary-bundle lifetime handle ends and response data begins.
    Current cleanup: display-oriented helper methods on app value types are moving back to CLI or
    runtime edges, so public request/response enums remain data shapes instead of mixed-in
    formatting utilities.
    Current cleanup: app-owned contract modules are now also split by domain, with
    `types/{install,addon,bundle,backup,runtime,external_package}` and
    `response/{installation,addon,addon_index,addon_lock,backup,bundle,external_package}`
    replacing the previous monolithic files. This keeps the stable app boundary easier to review,
    evolve, and bind from a future `egui` frontend.
    Current cleanup: `core::app::task_support` now also owns the shared direct/collecting/callback
    service-task wrappers, so addon/addon-index/addon-lock/backup/bundle/external-package services
    no longer repeat the same closure shells just to forward into their `*_task(...)` methods.
    Current cleanup: request contracts now follow the same domain split under
    `request/{installation,addon,addon_index,addon_lock,backup,bundle,external_package}`, and the
    remaining external-package `apply_runtime_defaults()` helpers are crate-visible again rather
    than public API. Runtime default projection stays inside app assembly.
    Current cleanup: runtime-backed request defaults now share `core::app::request` field helpers
    plus one `RuntimeDefaultableRequest` projection trait, so backup-output, backup-dir,
    bundle-output, and source-platform defaults no longer drift across request families.
    Current cleanup: app contract projections now share one private `core::app` collection helper,
    reducing repeated Vec projection code across request, value, and response boundaries without
    publishing another frontend-facing conversion trait.
    Current cleanup: app request contracts no longer expose public `From<app request> for domain`
    conversions. Crate-internal projection now lives on explicit `into_domain_*` helpers so the
    stable frontend boundary no longer advertises domain request types as part of its public trait
    surface.
    Current cleanup: app value contracts now follow the same rule. Domain projection is expressed
    through crate-internal `from_domain()` / `into_domain()` helpers instead of public
    `From<domain>` / `From<value>` trait impls, so CLI and services can still assemble domain
    requests internally without making those conversions part of the frontend-facing contract.
    Current cleanup: app flavor values no longer expose path-layout helpers publicly; CLI can still
    use crate-internal display slugs, while domain install code remains the owner of folder-name
    layout rules.
    Current cleanup: large `core::app` modules now also keep tests in sibling `*/tests.rs`
    submodules instead of interleaving fixtures with production code, which makes the stable app
    boundary easier to review while keeping app-level regression coverage intact.
    Current cleanup: those `core::app` sibling test modules now also import their owning service
    and contract types explicitly instead of depending on `super::*`, so app test coverage no
    longer relies on hidden parent-module import preludes either.
    Current cleanup: `core::app::request` and `core::app::response` child modules are now only
    visible to their parent module (`pub(super)`), and `core::app::mod` owns the single explicit
    request/result export whitelist instead of splitting that responsibility across duplicate
    aggregation lists or `response::*`.
    Current cleanup: `core::app::types` now follows the same rule; its child modules are only
    parent-visible, and `core::app::mod` owns the single public app-value export whitelist instead
    of routing value visibility through another `types::*` aggregation shell.
    Current cleanup: `core::bundle::{archive_read,apply_policy,target_accounts}` no longer
    re-export function sets from thin shell modules. Those modules now only delimit subdomains,
    while bundle internals import the concrete child modules they actually depend on.
    Current cleanup: `core::bundle::{shared,apply_model,execution,wtf_archive}` now follow the
    same rule. Shared path helpers, prepared-apply shapes, execution helpers, and WTF packers are
    imported from explicit child modules instead of shell-level re-export facades.
    Current cleanup: `core::bundle::{addon_source_archive,apply_source,entry_plan,planner}` now
    also use explicit child-module imports for addon-source archive helpers, apply-source reader
    state, entry planning, and planner pipeline functions instead of internal shell-level
    re-exports.
    Current cleanup: `core::bundle::apply` and `core::bundle::packing` now follow the same
    pattern internally, so non-public bundle code depends on their concrete child modules while
    `core::bundle::mod` remains the stable public export owner.
    Current cleanup: `core::bundle::{types,external_package}` now also only delimit
    parent-visible subdomains for internal callers, while `core::bundle::mod` owns the single
    public export whitelist for bundle DTOs and external-package API entrypoints.
    Current cleanup: the bundle root now re-exports its public API directly from owner modules and
    the transitional `core::bundle::exports` shell has been removed, so bundle contract changes no
    longer hide behind an extra wildcard export layer.
    Current cleanup: runtime path/default projection helpers are crate-internal again, and
    `ExtendedAppServices` now exposes an explicit `stable()` bridge instead of implicit `Deref`
    compatibility with the stable app boundary.
    Current cleanup: stable and extended app roots now also own constructed service instances
    directly instead of rebuilding fresh service wrappers from cloned runtime state on every
    internal accessor call, so the app boundary behaves more like a long-lived service contract
    and less like a service factory façade.
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
  Completed: long-running stable and extended app entrypoints now use one public task-shaped
  contract: callers receive `TaskRun<TResult>` directly instead of a triplet of direct,
  collecting-progress, and callback variants for the same operation. CLI convenience helpers now
  render `run.result` without widening the stable boundary again. Callback-based streaming still
  exists below the stable boundary on service/task-support helpers for internal reuse and targeted
  tests. Successful long-running tasks still begin with `Preparing`, end with `Completed`, and may
  report task-specific intermediate phases such as `Planning`, `BackingUp`, `Executing`, or
  `Verifying`. Those task-contract types are now also surfaced from `core::app`, so frontend
  callers do not need to import `core::task` separately just to consume app-service progress
  behavior.
- [x] keep optional provider/helper capability switches behind runtime/service boundaries instead of
  leaking them into CLI orchestration
  Completed: `AppRuntime` no longer requires addon-provider domain option types at the frontend
  boundary. Default provider cache/retry configuration now uses app-owned
  `AddonProviderOptionsValue` and `AddonProviderRetryPolicyValue`, custom provider injection is
  crate-internal, and helper strategy lives on runtime instead of bundle-domain plan DTOs.
  `AppRuntime`, `ExtendedAppServices`, and `StableAppServices` now expose one app-owned
  `AppRuntimeCapabilitiesValue` snapshot so frontend callers can read provider/helper capability
  state without inferring it from ad hoc `Option` semantics or planner details. Any future
  non-native helper capability should extend that runtime-owned contract rather than reappearing as
  an ambient planner concern.

Exit criteria:

- `core::app::StableAppServices` is the credible stable frontend root, while `ExtendedAppServices`
  remains the explicit extension root for less-stable app operations
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
  Current progress: external-package normalized-path collision rules now also have pure logical
  regression coverage in addition to zip-fixture coverage, so Windows/default-macOS case-folding
  behavior is tested without depending on a host filesystem that can physically materialize
  case-only directory entries.
  Current progress: author-package zip and directory analysis now also has explicit coverage for
  common macOS resource-fork metadata and desktop noise (`__MACOSX`, `._*`, `.DS_Store`,
  `Thumbs.db`, `desktop.ini`), so these archive artifacts stay out of normalized package entries
  and warning counts.
  Current progress: archive-compatibility coverage now also includes a larger wrapped author zip
  with a dozen addon roots, more than one hundred normalized files, WTF/fonts/interface resources,
  and repeated archive noise, so resource summary behavior is no longer only covered by tiny
  fixtures.
- [ ] verify the cleaned-up contracts against more Windows-to-macOS author-package scenarios
  Current progress: bundle apply planning now rejects planned target-path collisions using the
  target platform's case-sensitivity rules, so Windows/default-macOS targets fail fast when two
  archive entries differ only by case while Linux targets may still plan distinct paths.
  Current progress: addon-root discovery and prefix matching now also follow the same
  platform-aware contract. Windows/default-macOS author-package normalization and addon-archive
  install/update flows no longer silently drop files when one addon subtree uses mixed path casing,
  while Linux keeps case-sensitive distinct-root behavior.
  Current progress: the Windows-to-macOS apply floor now includes a complex author-package zip with
  a wrapper directory, mixed-case `Interface/AddOns` subtree paths, WTF/fonts/interface resources,
  default author-package apply policies, backup creation, and macOS/desktop noise that must not be
  imported.
- [ ] tighten any remaining path portability edge cases around case folding, archive metadata, and
  caller-working-directory assumptions
  Current progress: bundle export no longer defaults output paths or relative output references
  against the ambient process working directory, and relative bundle addon-index references now
  require an explicit `manifest_base_dir` instead of silently resolving against `cwd`. Remaining
  work is mainly broader archive-metadata hardening plus any other case-folding or ambient-path
  edges outside the bundle export path.
  Current progress: bundle addon-source archive names and embedded addon-index metadata file names
  now use case-insensitive uniqueness checks before zip output, so Windows/default-macOS bundles do
  not rely on the lower-level archive writer to catch metadata name collisions late.
  Current progress: addon-index package IDs are now validated with the same case-insensitive
  semantics used by index package lookup, so curated indexes cannot define ambiguous IDs such as
  `Details` and `details`.
  Current progress: installation path normalization now preserves Windows verbatim UNC roots when
  trimming `\\?\` prefixes, so `\\?\UNC\server\share\...` remains an absolute UNC path instead of
  degrading into a relative-looking `UNC\server\share\...` path.
  Current progress: external-package source analysis now rejects directory and zip symlink entries
  explicitly instead of treating symlink targets as portable regular-file payloads.
  Current progress: archive path validation now also rejects Windows-reserved segment characters,
  device names, and trailing-dot / trailing-space segments so portable bundles and author-package
  zips fail early instead of depending on target filesystem quirks.
  Current progress: bundle plain-name validation now also reuses the same shared portable
  single-segment rules, so addon names, account/server/character mapping names, interface asset
  names, and addon-index file names reject Windows-reserved names and trailing-dot /
  trailing-space segments before bundle planning/export depends on host filesystem behavior.
  Current progress: addon lock sidecar source-index paths now also reuse the shared zip-segment
  validation floor, so malformed `sources.toml` entries with empty, reserved, or otherwise
  non-portable segments fail consistently before sidecar path resolution.
  Current progress: addon lock sidecar source-index resolution and bundle embedded addon-source
  index extraction now also share one rooted `sources/` path helper under `core::archive_path`,
  while still preserving caller-specific error context instead of keeping parallel root checks.
  Current progress: zip archive entry validation now also shares one helper in `core::archive_io`
  for symlink rejection plus non-directory portable-path validation, and addon archive prep /
  external-package zip ingest now also reuse the same helper to return owned file segments instead
  of validating and reparsing the same entry name separately.
  Current progress: external-package staging materialization now also reuses the shared
  `core::bundle::entry_layout` classifier for normalized bundle-style paths, so addon/WTF/fonts/
  interface root dispatch is no longer maintained in a second parallel match tree when building a
  first-party bundle from an author package.
  Current progress: normalized external-package entries now also derive `group`, `wtf_scope`, and
  source account/server/character identity from the shared `core::bundle::entry_layout`
  classifier, so author-package analysis no longer keeps a second semantic mapping from
  normalized bundle paths to apply-group and WTF-scope meaning.
  Current progress: external-package warning taxonomy no longer carries the unreachable
  `unsupported_wtf_root_savedvariables` code; root `WTF/Account/SavedVariables/<file>` entries are
  recognized as supported common root saved variables, while the remaining warning codes now map to
  real unsupported layouts only.
  Current progress: external-package WTF source classification now splits pure suffix-to-layout
  recognition from warning rendering and entry assembly, so author-package analysis can evolve WTF
  layout rules without re-threading source-path-specific warning text through the core layout logic.
  Current progress: external-package non-WTF rooted path handling now also splits pure
  fonts/interface/AddOns subtree recognition from entry assembly, so author-package analysis can
  test resource-root semantics without coupling every branch to `ExternalPackageEntry` creation or
  warning-object construction.
  Current progress: external-package addon classification now also splits pure addon-root /
  missing-root semantics from entry assembly and warning creation, so addon-path normalization,
  addon-subtree warning behavior, and `.toc`-driven root discovery can evolve independently inside
  one source-to-normalized-path pipeline.
  Current progress: backup restore archive preparation now also parses each entry name into one
  shared `group + destination` result instead of reparsing the same zip path separately for group
  membership and target-path resolution, while preserving the distinction between unsupported root
  entries and root-only unsupported entry paths.
  Current progress: backup archive root naming now also lives with `BackupGroup`, so backup group
  metadata labels (`interface_assets`) stay distinct from archive root names (`interface`) while
  creation-time root emission and restore-time root parsing both reuse the same group-owned
  contract instead of keeping parallel hardcoded root strings.
  Current progress: backup restore archive preparation now rejects symlink entries before any
  transactional restore work begins and reuses the same portable archive-segment validation as
  bundle/external-package ingest, so restore safety no longer lags behind import safety.
  Current progress: backup creation now also rejects local directory/interface symlink entries
  instead of following link targets into the archive, so backup payloads stay portable and bounded
  to the intended WoW tree.
  Current progress: addon local-archive package preparation now also rejects zip symlink entries,
  so first-party addon install/update flows share the same archive-metadata safety floor as backup
  restore and external-package ingest.
  Current progress: addon local-archive preparation now also validates staged addon-directory and
  file target collisions using the selected target platform's case-sensitivity rules, so
  Windows/default-macOS installs fail fast on case-only archive conflicts instead of depending on
  host filesystem behavior during extraction.
  Current progress: addon registry load/save now validates schema version, non-empty package IDs
  and addon-directory sets, case-insensitive package IDs, and case-insensitive addon-directory
  ownership before state is trusted or persisted. Addon-index bulk matching now also records used
  tracked package IDs with the same normalized key, so case-folded registries cannot make one
  package appear available twice during curator attach/update flows.
  Current progress: addon install planning and addon mutation execution now resolve existing
  `Interface/AddOns` entries with the selected installation platform's case-folding rules. Dry-run
  installs for Windows/default-macOS targets now reject `Details` vs `details` conflicts before
  backup or execution, and replace/update/remove paths delete the actual on-disk entry instead of
  assuming the archive casing matches the live directory casing.
  Current progress: addon-lock planning now uses the same platform-aware addon-directory keys for
  tracked owners, freed directories, and untracked replace checks. macOS/Windows plans treat
  case-only live AddOns as conflicts, while Linux plans can still keep case-distinct directories.
  Addon-lock verification also resolves tracked addon directories through the selected platform
  before reporting missing directories, so case-only live casing drift is not misclassified as a
  vanished addon on default macOS/Windows targets.
  Current progress: addon local-archive source paths now have an explicit frontend/core boundary.
  CLI runtime assembly records the invocation directory as the absolute app relative-path base, app
  addon install/relink requests resolve relative local zip sources against that base, and persisted
  registry sources, addon-lock local archive source refs, and explicit addon-lock source overrides
  must already be absolute before they reach core planning or materialization. Future GUI callers
  can set their own runtime base instead of inheriting process cwd accidentally.
  Current progress: addon-index file paths now follow the same app-boundary rule. Relative
  `addon index` app requests resolve against the runtime relative-path base before inspect,
  validate, suggest, scaffold, attach, install, update, or relink touches the filesystem, while
  index-internal local archive sources remain explicitly index-relative.
  Current progress: bundle archive inputs, external-package source paths, config source paths, and
  bundle manifest base directories now also resolve at the app boundary. The bundle-domain output
  placement rule remains separate, so relative bundle export destinations still resolve against
  the manifest/installation output base instead of the app relative-path base.
  Current progress: addon-lock and backup selection paths now also resolve at the app boundary.
  Relative lock diff files, verify/plan/apply lock files, explicit addon-lock source override
  archives, backup list directories, restore directories, and restore archive paths resolve against
  the runtime relative-path base before core code reads them.
  Current progress: app-level output choices now use the same app-boundary rule. Mutation backup
  output directories, backup creation output directories, addon adoption archive outputs, and
  external-package bundle output directories resolve against the runtime relative-path base before
  core services write files. Bundle pack output paths remain the explicit exception and keep the
  manifest/installation-derived bundle-domain placement rule.
  Current progress: installation path inputs now also resolve at the app boundary. Relative
  inspect/resolve installation paths and configured installation scan roots are joined to the
  runtime relative-path base before install-discovery core code probes the filesystem.
  Current progress: CLI-only sidecar files now also resolve through the runtime base before CLI
  loaders read them. Bundle pack manifests, manifest validation files, and apply mapping files no
  longer reintroduce process-cwd filesystem reads outside app services.
  Current progress: addon cache runtime paths are now absolute after boundary handling. CLI
  one-shot cache overrides resolve against the invocation runtime base,
  `settings set --addon-cache-dir` persists resolved absolute paths, and relative persisted cache
  paths fail closed.
  Current progress: runtime construction now has a fallible `AppRuntimeBuilder` for path-bearing
  production assembly, so addon provider options, scan roots, and default output directories
  normalize after the relative-path base is known instead of depending on constructor call order.
  Current progress: runtime-owned path policy is now immutable after build. Path-bearing
  `AppRuntime` mutators were removed, so CLI, future GUI code, and tests all use the fallible
  builder path for scan roots, runtime base, and default output directories.
  Current progress: addon-root matching now also prefers exact-prefix matches but falls back to
  case-insensitive matching on Windows/default macOS targets, so mixed-case archive subtrees stage
  into the intended addon root instead of being skipped as if they belonged to no addon.
  Current progress: cross-platform target-path collision detection now also shares one canonical
  helper under `core::archive_path`, and backup restore reuses the same case-folding rules as
  addon prep, bundle planning, and external-package normalization instead of carrying a separate
  duplicate implementation.
  Current progress: the same shared path-safety layer now also detects file/directory ancestor
  conflicts with platform-aware case folding, so addon prep, bundle planning, backup restore, and
  external-package normalization all reject target hierarchies that would collide only on
  Windows/default-macOS filesystems.
  Current progress: zip-style archive path serialization now also lives in the same shared core
  helper layer, so backup zip writing and bundle zip writing no longer maintain separate
  slash-normalization implementations.
  Current progress: zip-writing entry creation now also reuses shared portable-path validation in
  `core::archive_io`, so backup creation and bundle packing reject non-portable archive names
  before emitting first-party zip payloads.
  Current progress: first-party backup/bundle export flows now also maintain one shared archive
  output path set while writing, so case-only duplicate names and file/directory hierarchy
  conflicts fail before any non-portable first-party zip layout is emitted.
  Current progress: first-party backup and bundle export output tracking now also has focused
  pure-logic regression coverage for case-only metadata collisions, file-as-ancestor conflicts,
  and legal directory ancestors, including bundle addon sidecar metadata/source paths.
  Current progress: first-party backup and bundle export now also share the same archive-output
  issue-to-error mapping helper in `core::archive_io`, so duplicate collision/prefix-conflict
  wording no longer lives in parallel wrappers.
  Current progress: unsupported-symlink rejection now also shares one canonical helper in
  `core::archive_io`, so addon archive prep, backup restore/source scanning, bundle archive
  ingest, and external-package ingest no longer keep separate identical error branches.
  Current progress: external-package directory sources now reuse the same portable segment
  validation as zip sources, so reserved Windows names, trailing-dot/space segments, and similar
  non-portable entries fail consistently before normalization.
  Current progress: bundle archive inspect/apply/addon-lock extraction now also reject symlink
  entries up front, so first-party bundle ingestion no longer trails addon/external-package/backup
  archive safety.
  Current progress: bundle archive entry validation now also rejects non-portable path segments
  during inspect/apply, so first-party bundle archives fail on the same portable-name floor as
  addon/external-package/backup archives.

Exit criteria:

- Windows and macOS callers share one deterministic import contract
- helper-assisted paths, if added later, remain optional accelerators rather than architecture owners

## R5 - Operational Correctness and Update Freshness

Goal: the next fearless-refactor pass should fix the correctness gaps that would otherwise make
addon update, addon-lock sync, and frontend-facing dry-run behavior feel trustworthy only in happy-path tests.

- [x] define source freshness semantics for mutable remote references versus immutable pinned artifacts
  Completed: provider download caching now distinguishes mutable versus resolved immutable remote
  artifacts. Floating GitHub releases and floating CurseForge mod references resolve to a concrete
  tag or file id before cache reuse, while raw `http(s)` archives refresh even when a cache
  directory is configured.
- [x] make addon update capable of refetching newer remote artifacts when the recorded source
  reference is mutable
  Completed: repeated `addon update` runs no longer freeze floating remote references behind a
  stale cache hit, so a newly resolved GitHub release tag or CurseForge file can be observed.
- [x] make addon-lock apply transaction-complete, including post-apply verification and
  cancellation semantics
  Completed: addon-lock apply now treats “execution succeeded but verification could not complete”
  as a rollback path when a backup exists. Verification cancellation or verification-time errors no
  longer leave filesystem mutations in place while still returning failure to the caller.
- [x] decide whether default public planning stays rewrite-aware and compare-heavy or whether the
  product should split logical planning from deeper content-compare preview
  Completed: default public plan is now logical and conservative. `bundle plan` and
  `external-package plan` no longer open source readers or byte-compare existing targets just to
  decide `skip` versus `replace`; existing-target entries are planned as replace candidates, while
  exact identical-file skipping remains part of prepare/apply execution paths.
- [x] harden URL-derived archive naming and remote download fallback naming rules for cross-platform inputs
  Completed: URL-derived file-name guessing now ignores query strings and fragments instead of
  treating the whole URL as a filesystem path.
- [x] make addon registry and addon-lock state-file writes atomic
  Completed: `addons.toml` and `lock.toml` now write through same-directory temporary files with
  flush/sync before atomic replacement, so normal state persistence no longer truncates the active
  registry or lock file on mid-write interruption.
- [x] record these issues as part of the current core stream instead of opening a parallel
  workstream
  Completed: `review-2026-04-21.md` is the bounded review note for this slice.

Exit criteria:

- mutable remote addon sources have explicit refresh rules and reliable update behavior
- addon-lock apply reports one coherent success/rollback/failure outcome across execute and verify phases
- the public plan contract is explicit that default public planning is logical and conservative,
  while exact identical-file skipping belongs to prepare/apply
- remote download naming stays portable across Windows and macOS

## R6 - Structured Task Progress Contract

Goal: keep the stable task contract human-readable for CLI while making progress streams structured
enough for future `egui` work to consume directly.

- [x] add stable task identity to collected-progress and callback task streams
  Completed: `TaskRun` now carries one generated `task_id`, and wrapper-generated
  `TaskProgressEvent` payloads reuse that same id for collecting-progress and callback flows.
- [x] extend `TaskProgressEvent` with optional machine-readable fields instead of replacing the
  existing message string
  Completed: task progress events now keep `message` while also exposing optional `code`,
  `current`, `total`, `bytes_current`, `bytes_total`, and `bytes_per_second` fields.
- [x] keep task id generation and common progress shaping inside shared task infrastructure rather
  than pushing it into each business operation
  Completed: `core::task` now owns task id generation plus shared progress emit helpers, so
  frontend-visible identity and payload shape do not depend on ad hoc service logic.
- [x] convert key execution loops to structured step progress instead of text-only updates
  Completed: addon directory mutation, metadata-only addon-lock actions, backup restore execution,
  and bundle apply operation loops now emit typed codes plus `current/total` counts.
- [x] add regression coverage at both the task layer and app boundary
  Completed: `core::task` now validates generated task ids plus structured step/byte fields, and
  app-layer addon, backup, and external-package tests verify that stable services forward the same
  structured task contract.
- [x] wire provider-backed archive downloads into real byte progress without introducing a second
  task contract
  Completed: provider HTTP downloads now emit throttled byte progress through
  `TaskProgressCode::DownloadArchive`, and addon install/update, addon-index install/update, and
  addon-lock source preparation now forward those bytes through the stable app task stream.
- [x] keep this work inside `wow-addon-sync-core` instead of opening a parallel frontend workstream
  Completed: the task-contract slice extends the same reusable core boundary that CLI and future
  `egui` code both depend on.

Exit criteria:

- frontend callers can correlate one logical task across ordered collected-progress or callback
  events without inferring identity from message text
- step-oriented execution loops can expose deterministic counts through shared task helpers
- provider-backed download phases can expose byte-level transfer progress through the same
  app-facing event shape without another contract break

## R7 - Config Sync Ergonomics and Policy Separation

Goal: keep the current reusable core model, but close the remaining product-shaping gaps before
`egui` turns them into public UI debt.

- [x] harden `lua_patch` text rewriting so the text path matches the byte path's placeholder-safety floor
  Completed: `src/core/lua_patch/text.rs` now uses unique placeholder generation instead of fixed
  `__HEARTHSYNC_REWRITE_<n>__` literals, and regression coverage now verifies that pre-existing
  placeholder text in SavedVariables content is preserved instead of being rewritten accidentally.
- [x] add a first-class config-oriented command and app surface without creating a second planning engine
  Completed: the CLI now exposes explicit `config inspect`, `config plan`, and `config apply`
  commands, and `core::app::StableAppServices` now also exposes config-oriented request and task
  entrypoints on top of the same external-package planning/apply engine instead of requiring
  frontend callers to stay on external-package-shaped app contracts.
- [x] introduce a separate addon policy/preferences layer instead of overloading `lock.toml`
  Completed: addon mutable preferences now persist in
  the managed addon state backend (default platform app-data; sidecar portable mode optional), the
  stable app boundary exposes
  inspect/set/remove addon-policy entrypoints, and the CLI now has a first-class
  `addon policy inspect|set|remove` surface instead of forcing future `egui` work to overload the
  reproducible lock model.
  Current progress: bulk `addon update` and bulk `addon index update` now honor `ignored = true`,
  provider-backed addon update now also applies basic pin overrides without changing the tracked
  package identity (`pin.file_id` for CurseForge, `pin.version` as a GitHub tag override), and
  all-ignored update runs now short-circuit without creating pointless backups.
  Current progress: regular provider-backed `addon update` now also forwards
  `release_channel` / `allow_prerelease` into provider resolution without changing tracked source
  identity. Floating GitHub releases treat `allow_prerelease = true` or
  `release_channel = beta|alpha` as prerelease-eligible selection, while floating CurseForge mods
  now map `stable|beta|alpha` onto release-type filtering. Explicit GitHub `tag` pins and
  CurseForge `file_id` pins remain authoritative and bypass floating release-channel filtering.
  Current progress: regular provider-backed `addon update` now also consumes
  `install_dependencies = true` for supported sources by installing missing required CurseForge
  dependencies as additional tracked packages during the same update transaction. Unsupported
  source kinds now fail explicitly instead of silently treating the field as a no-op.
  Current progress: `addon index update` now also consumes `install_dependencies = true` for the
  same first dependency-installation slice, but it resolves dependencies from the curated index
  source while keeping that source authoritative instead of letting mutable policy rewrite it.
  Unsupported source kinds fail explicitly there too instead of acting like a silent no-op.
  Current cleanup: addon-policy execution now also projects regular provider-backed update policy
  separately from indexed-update policy, so pin and release-channel/prerelease overrides stay
  structurally unavailable to `addon index update` instead of only being excluded by convention.
  Current cleanup: provider-side dependency resolution no longer returns a bare source list. It now
  returns an explicit dependency-resolution strategy value, so the current
  `missing required only` behavior is represented in code instead of being inferred from comments
  and call-site assumptions.
  Current cleanup: provider-side dependency support also has an explicit capability surface now.
  Unsupported sources and `missing required only` sources are distinguished by code contract before
  execution starts instead of only surfacing through downstream validation messages.
  Current cleanup: that dependency capability now also projects through app-owned source values, so
  stable app callers can read source-level dependency support from addon inventory/search/index/lock
  results before they trigger an update flow.
  Current cleanup: addon and addon-index app services now also preflight
  `install_dependencies = true` against provider dependency capability before they enter domain
  update execution, so unsupported source kinds fail from the app boundary instead of only through
  deeper prepare-stage validation.
  Current cleanup: addon-index schema now also supports exact author-declared
  `match_package_ids` hints, and both preflight and domain matching reuse them before falling back
  to weaker continuity signals. This closes the remaining "source family changed, no directory
  hints, no stable display name, but the curator knows the old tracked package id" gap without
  adding new fuzzy matching.
  Remaining gap: dependency execution is still intentionally narrow. Indexed update still does not
  consume pin or release-channel/prerelease source-resolution overrides, and dependency
  installation is not yet a generic cross-provider or dependency-upgrade policy.
  Remaining gap: addon-index preflight matching is still intentionally conservative when the
  curated source family itself changes and the index package omits exact `match_package_ids`,
  stable `addon_directories`, and unique exact display-name continuity. Metadata, package-id,
  curated exact package-id hints, exact source identity, provider-level source-family identity,
  and unique exact display-name continuity cases now preflight cleanly, but the narrower
  no-hint/no-directory/no-display-continuity case still falls back to the existing domain
  validation path.
  Current cleanup: that remaining fallback path now also emits explicit operator guidance when an
  unsupported dependency-install policy is only discovered during domain preparation. The error now
  explains that app preflight could not lock the package mapping from stable identity hints alone
  and points index curators toward exact `match_package_ids`, stable `addon_directories`, or
  unique exact package-name continuity instead of leaving the fallback as a generic unsupported
  source failure.
  Current cleanup: `addon index inspect` now also exposes structured exact-identity-hint coverage
  instead of making operators infer it from raw package fields. CLI and future GUI callers can see
  how many packages have both exact hint types, how many packages still omit
  `match_package_ids` or `addon_directories`, and which package ids still lack any exact
  migration hint entirely.
  Current cleanup: that inspect surface now also emits structured warning objects for packages that
  still have exact-identity-hint gaps, with stable warning codes, explicit
  blocking-versus-advisory severity, package ids, and operator-facing messages. JSON, CLI, and
  future GUI consumers no longer need to reverse-engineer risk only from coverage counters or
  scrape severity from human text.
  Current cleanup: warning taxonomy is now explicit instead of collapsing every issue into one
  bucket. `missing_exact_identity_hints` remains the blocking "both hints absent" case, while
  `missing_match_package_ids` and `missing_addon_directories` stay advisory so curators can see
  incomplete exact-hint coverage without turning every partial hint into a hard validation failure.
  Current cleanup: `addon index validate` now turns those structured inspect warnings into an
  explicit curator gate. It returns a structured validation result for JSON/app consumers,
  including total, blocking, and advisory warning counts, while the CLI only exits non-zero when
  the index still has blocking curator warnings.
  Current cleanup: addon-index curation now also has a first authoring helper instead of only
  diagnostics. `addon index suggest` reuses the existing preflight matching order against the
  current tracked registry and suggests missing `match_package_ids` and `addon_directories`
  additions without adding weaker runtime matching rules.
  Current cleanup: that suggestion surface reports `suggested`, `complete`, `no_local_match`, and
  `ambiguous_local_match` per package, together with the match strategy that justified the current
  local mapping, so future GUI work can present curator assistance without scraping CLI text.
  Current cleanup: the same helper now also resolves local-archive index sources through the same
  index-relative canonicalization path used by install/update, so local authoring workflows do not
  miss source-identity matches just because tracked registry sources are stored canonically.
  Current cleanup: addon-index authoring now also has a first bootstrap path. `addon index
  scaffold` writes a new curated index from the current tracked addon registry instead of forcing
  operators to hand-copy source references out of local state.
  Current cleanup: that scaffold path preserves existing curated metadata when present
  (`index_package_id`, package name, version, source URLs, hash, supported flavors), emits tracked
  addon directories directly, and only adds `match_package_ids` when the preserved curated package
  id differs from the tracked package id.
  Current cleanup: scaffold is intentionally fail-closed. It refuses to overwrite an existing index
  without `overwrite = true`, and it rejects installations that have no tracked addon registry yet
  instead of inventing source records from an untracked addon folder scan.
- [x] make provider cache reuse integrity-aware rather than file-presence-aware
  Current progress: immutable-source cache reuse now requires a local cache-integrity sidecar, and
  existing cached archives are re-hashed before reuse so missing sidecars or locally modified cache
  files trigger a re-download instead of blind reuse.
  Current progress: resolved GitHub release assets and CurseForge files now also feed remote
  validator metadata into cache reuse decisions. Cached immutable artifacts are refreshed when
  provider-side size, modified time, or published digest/hash metadata changes even if the local
  archive still matches the previous sidecar.
  Current progress: generic `http(s)` archives now also use one shared transport-level
  conditional-request path through the provider HTTP layer. When a cached archive still matches
  the local integrity sidecar and the server previously exposed stable `ETag` or `Last-Modified`
  headers, HearthSync now sends `If-None-Match` / `If-Modified-Since` and reuses the cached
  archive on `304 Not Modified`; otherwise it refreshes the download and records the new transport
  validators from the successful response.
  Current progress: arbitrary archive URLs that do not expose reusable transport validators no
  longer have to choose between blind long-term reuse and refresh-on-every-run. Cache sidecars now
  record fetch time, the default provider policy reuses those cache entries only within an
  explicit short freshness window, and legacy sidecars that predate the fetch-time field still
  fail closed by refreshing.
  Current progress: provider, stable app, and CLI now also expose first operator-facing
  `addon cache purge|repair` flows. `purge` clears the configured download cache root, while
  `repair` removes incomplete downloads, orphaned archives, missing-archive sidecars, and
  integrity-mismatched cache entries without inventing a second cache-state model.
  Current progress: that repair path now also performs first remote cache validation instead of
  staying local-only. HTTP archives with stored `ETag` / `Last-Modified` metadata use conditional
  GET during repair, resolved GitHub/CurseForge artifacts compare fresh provider-side validators
  before deciding whether to refresh, and no-validator HTTP entries that already exceed the
  configured freshness window are pruned instead of lingering indefinitely.
  Current progress: the provider cache contract is now also operator-facing instead of staying
  test-only. CLI/runtime now accept explicit global cache configuration such as `--addon-cache-dir`,
  `--addon-http-no-validator-window-secs`, and
  `--addon-http-no-validator-always-refresh`, and runtime diagnostics expose the resulting policy
  through the stable app capability surface.
  Current progress: cache and addon-state runtime defaults now also have one explicit persistent
  settings backend. `core::app` persists selected runtime overrides under app-data
  `settings/runtime.toml`, the CLI exposes `settings inspect|set|reset`, and runtime assembly now
  applies persisted settings before one-shot CLI flag overrides.
  Follow-up gap: remote repair is still best-effort rather than a richer operator-configurable
  policy surface, and future GUI work still needs a richer settings surface on top of the shared
  backend instead of reintroducing CLI-only persistence.
- [ ] add sanitized real-world SavedVariables fixture coverage before broad desktop-facing config claims
  Current progress: `core::lua_patch` now includes first sanitized fixture-style coverage for a
  more realistic UTF-8 `Details.lua` payload with Chinese text plus a more realistic Latin-1
  `Pawn.lua` payload with extended characters, so localized text preservation and non-UTF-8
  identity rewrites are no longer only covered by tiny synthetic snippets.
  Current progress: fixture breadth now also covers a more realistic UTF-8 `Clique.lua` slice with
  Chinese character/server profile keys plus a more realistic UTF-8 `BagSync.lua` slice with
  realm-and-character keyed account data, so rewrite coverage no longer depends only on one
  profile-key addon shape and one Latin-1 character-root addon shape.
  Current progress: fixture breadth now also covers a more realistic UTF-8 `AddOnSkins.lua` slice
  with `profiles` plus `profileKeys` keyed by `角色 - 服务器`, so the positive floor now also
  includes a profile-key-driven addon that does not rely on the broader identity whitelist.
  Current progress: fixture breadth now also covers a more realistic UTF-8 `ElvUI.lua` slice with
  mixed `角色 - 服务器` profile keys, nested realm/character maps, and no-space
  `角色-服务器` combined identity keys, so the rewrite floor now includes the current
  `lua_patch` support for both spaced and compact combined identities.
  Current progress: fixture breadth now also covers a more realistic UTF-8 `NewBeeBox.lua` slice
  with no-space `服务器-角色` combined identity keys plus separate `name` / `realmName` fields, so
  the rewrite floor now also includes the current narrow support for reversed compact combined
  identities without touching `Player-...` GUID fields.
  Current progress: identity rewrite targeting is now more conservative too. Unknown
  `SavedVariables/*.lua` files no longer enter identity rewrite only because they contain generic
  `playerName` / `realm`-style field names; identity rewrite now stays on explicit known-file
  rules, which keeps real multi-character account files such as `Syndicator.lua` fail-closed by
  default.
  Current progress: the same fail-closed boundary is now explicitly regression-covered for
  `WIM.lua`-style chat history and cache payloads. Nested `服务器 -> 角色 -> 会话历史` data is
  treated as out of scope for automatic config rewrite instead of being folded into the supported
  configuration surface.
  Current progress: `Rarity.lua` is now narrower too. HearthSync still rewrites `profileKeys`
  through the generic profile-key signal path, but it no longer treats `Rarity.lua` account-wide
  statistics as identity-rewrite-safe because real payloads contain many same-server characters and
  broad `playerName` / `server` replacement would mis-target unrelated records.
  Current progress: fixture breadth now also covers a more realistic UTF-8 `RurutiaSuite.lua`
  slice plus a more realistic UTF-8 `NDui_Bags.lua` slice with a long single-line
  `profileKeys` payload, so the generic profile-key path is now regression-covered against real
  addon author text, mixed simplified/traditional character variants, legacy suffixed profile keys,
  and dense one-line account maps without widening the identity whitelist.
  Current progress: the UTF-8 `profileKeys` path is now narrower too. HearthSync scopes
  profile-style rewrites to direct `profileKeys` entries, direct `profiles` keys, and
  `*profileKey` field values instead of doing whole-document text replacement for every exact
  `角色 - 服务器` match. Real `RurutiaSuite.lua` author text now stays untouched, and `Clique.lua`
  now also rides an explicit known-file identity rule because its real payload keeps per-character
  `char` tables keyed by `角色 - 服务器`.
  Current progress: the UTF-8 known-file identity path is narrower too. Explicit identity field
  values such as `playerName`, `realm`, `server`, `character`, `LastPlayerFullName`, `LastRealm`,
  `guildrealm`, `realmKey`, `rwsKey`, and paired `name + realmName` no longer rewrite through
  whole-document quoted-string replacement. Real `Details.lua`-style `lastPlayerName` text now
  stays untouched while real `NewBeeBox.lua` player records still rewrite correctly.
  Current progress: the UTF-8 identity-key path is narrower again. Exact identity-shaped Lua keys
  now rewrite only in known containers such as root table-key records, `profileKeys`, `profiles`,
  `char`, `faction`, `worldBoss`, `searchHistoryList`, `Toons`, `value`, `currentrealm`, and
  `totals`, plus root/faction `服务器 -> 角色` maps. Arbitrary nested cache keys that merely equal
  `角色 - 服务器`, `角色-服务器`, or `服务器-角色` now stay untouched.
  Current progress: the Lua byte fallback is no longer a whole-document text replacement path. It
  now uses the same Lua-structure scope as the UTF-8 path for `profileKeys`, `profiles`,
  `*profileKey`, known identity fields, name/realm pairs, known identity-key containers, and
  root/faction realm-character maps while preserving UTF-8 and Latin-1 byte encodings.
  Current progress: UTF-8 payloads no longer fall through to byte rewriting after a scoped text
  rewrite misses, so known-file identity rules cannot accidentally revive broad quoted-string
  replacement on ordinary UTF-8 SavedVariables text.
  Current progress: fixture breadth now also covers a realistic BigWigs-style profile-key payload
  where `profileKeys` should migrate but descriptive boss/profile notes must remain untouched, plus
  a realistic Baganator-style recent-character cache that stays fail-closed instead of rewriting
  history strings that merely look like character identities.
  Remaining gap: new addon-specific identity-key containers still require explicit evidence before
  they should join the shared container allowlist.
  Remaining gap: fixture breadth is still limited. More addon-specific SavedVariables shapes and
  more encoding/pathology variants are still needed before broad desktop-facing migration claims.

Exit criteria:

- configuration sync can be presented to users as a first-class product flow without creating a
  parallel config engine
- addon reproducibility state and mutable policy state are clearly separated
- cached source archives have explicit integrity behavior
- Lua rewrite safety is backed by stronger regression coverage on realistic inputs

## M8 - Tracked Registry Bootstrap for Existing Installs

Goal: existing manual addon installs should be able to enter tracked HearthSync state without
inventing remote provider identity, adding a self-referential local-directory source kind, or
silently sweeping the whole `Interface/AddOns` directory into fake packages.

- [x] keep this gap inside `wow-addon-sync-core` instead of creating a new workstream
  Completed: the bootstrap problem is now recorded as a bounded core milestone because it cuts
  across source semantics, registry ownership, curator tooling, CLI/app entrypoints, and future
  GUI reuse.
- [x] require explicit addon-directory selection for bootstrap instead of ambient full-folder scans
  Completed: the new bootstrap direction is explicit `addon adopt`, which accepts one or more
  explicit untracked addon directories and refuses to infer package grouping from the whole local
  AddOns tree.
- [x] represent adopted local state as a real snapshot archive inside the existing source model
  Completed: `addon adopt` now snapshots the selected addon directories into a real local archive
  (defaulting under the selected installation's HearthSync app-data addon state) and records that archive as the
  tracked `local_archive` source instead of inventing remote provider identity or adding a
  self-referential live-directory source kind.
  Current cleanup: multi-addon adoption now requires an explicit tracked `package_id`, duplicate or
  already-tracked addon directories are rejected, and archive output paths fail closed when they
  already exist or are placed inside one of the adopted addon directories.
- [x] move managed addon state defaults out of the WoW client tree while keeping sidecar optional
  Completed: addon registry, addon lock, addon policy, and adopted snapshot archives now resolve
  through one runtime-owned state-layout abstraction. Desktop defaults write to platform app-data
  keyed by installation identity, while sidecar `.hearthsync` remains available as an explicit
  portable backend instead of the product default.
  Current cleanup: the CLI now also exposes one global `--addon-state-storage app-data|sidecar`
  runtime switch, so operators can choose between desktop-style app-data state and portable
  sidecar state without reintroducing feature-specific path flags.
  Current cleanup: bundle pack plus `bundle addon-plan` / `bundle addon-apply` now also honor that
  runtime-owned backend when they read or apply active tracked addon state, instead of silently
  hard-coding default app-data semantics inside the bundle domain.
  Current cleanup: the stable app/runtime capability surface now also exposes addon-management
  semantics directly through app-owned values, including the selected addon-state backend plus the
  explicit `scan-only without managed state` and `managed mode requires state` product contract for
  future GUI callers.
  Current cleanup: the CLI now also has a first runtime-diagnostics query entrypoint, so operators
  and future GUI integration can read the stable runtime projection directly instead of inferring
  it from scattered command help or debug output.
- [x] add a minimal post-adopt source relink path that upgrades tracked snapshot state without
  reinstalling files
  Completed: `addon relink` now lets operators point one tracked package at a new
  provider-backed or archive-backed source, validates that the prepared source exposes the exact
  same addon-directory set, and then rewrites only the tracked registry source.
  Completed: generic relink clears stored package metadata rather than keeping stale index/source
  details that no longer truthfully describe the new source.
  Remaining rule: generic relink intentionally stays conservative. It still requires exact
  addon-directory parity and remains the source-only path; curated metadata attach now lives in
  `addon index relink`.
- [x] add curated/index-aware relink on top of the generic source relink path
  Completed: `addon index relink` now resolves one curated package from an addon index, matches or
  explicitly targets one tracked package, validates exact addon-directory parity, and then rewrites
  registry `source` plus curated metadata without reinstalling live addon files.
  Completed: index relink intentionally allows "metadata attach only" flows where the resolved
  source is already the same as the tracked package source but curator metadata is still missing.
- [x] decide whether single-package relink should grow into a guided bulk curator attach flow
  Completed: `addon index attach` now plans or applies many curator-aware relinks against one
  index without reinstalling live AddOns content. It reuses the same tracked-package matching
  order as `addon index suggest`, keeps exact addon-directory parity as the shared safety rule, and
  writes registry source plus curated metadata only when every selected package is ready.
  Completed: bulk attach is deliberately fail-closed by default. Mixed runs can still be previewed,
  and default non-dry execution does not partially write the registry when any selected package
  remains unmatched, ambiguous, or directory-incompatible.
- [x] decide whether bulk curator attach should later support operator-approved partial apply
  Completed: default `addon index attach` execution remains all-or-nothing, but operators can now
  opt into explicit ready-only partial apply. `apply_ready_only` writes only packages that planned
  as ready, keeps blocked packages in the result, reports `partial_apply`, and leaves the default
  path fail-closed for safer bulk curation.
- [x] normalize the platform app-data root naming before managed-state layout becomes migration-stable
  Completed: the canonical app-data root now resolves through application-only
  `ProjectDirs::from("", "", "hearthsync")`, which removes the duplicated
  `.../hearthsync/hearthsync/data/...` segment on Windows. Addon state and backup defaults now both
  use that shared helper, and the pre-release compatibility branch for the old layout has been
  deleted instead of being preserved.

Exit criteria:

- manual addon installs can become tracked without hand-writing `addons.toml`
- bootstrap does not rely on fake remote source reconstruction or silent whole-folder grouping
- curator scaffold/suggest flows can start from explicit adopted state on a real machine

## R9 - Pre-GUI Architecture Hardening

Goal: close the current review findings that could become expensive frontend contracts if left in
place.

- [x] record the 2026-04-28 architecture-hardening review inside `wow-addon-sync-core`
  Completed: `review-2026-04-28-architecture-hardening.md` keeps the findings in the core
  workstream instead of creating a duplicate top-level workstream.
- [x] make addon update and dependency installation one rollback-aware mutation outcome
  Completed: regular addon update and addon-index update now execute primary package updates plus
  dependency installs through one shared mutation helper that writes the addon registry only after
  every post-backup file mutation succeeds. Dependency install errors now enter the same rollback
  path as primary update errors, and regression coverage verifies that both files and tracked
  registry state return to the pre-update package set.
- [x] move bulk `addon index update` ignored-policy checks before package preparation when preflight
  matching is sufficient
  Completed: bulk addon-index update now applies ignored policy from preflight-matched tracked
  packages before resolving or preparing the source. Explicit named update still overrides ignored,
  and the fallback path remains available when matching cannot safely be determined before
  preparation.
- [x] replace config response aliases with config-owned app DTOs
  Completed: `ConfigInspectionResult`, `ConfigApplyPlanResult`, and `ConfigApplyResult` are now
  concrete app-owned DTOs with config-named nested entry, summary, warning, and source-kind types.
  `ConfigService` still reuses the external-package engine internally, but it projects the results
  at the app boundary so CLI and future GUI callers do not consume external-package result aliases.
- [x] decompose addon provider responsibilities into cache, materialization, validation, and
  source-adapter modules
  Completed: `src/core/addon/provider/mod.rs` now keeps the provider contract and default-provider
  composition, while cache maintenance, source materialization, remote validator construction,
  source identity helpers, and provider-adapter policy/search/dependency logic live in focused
  sibling modules. Provider cache/freshness changes and new source adapters no longer expand one
  multi-responsibility module. The provider-local clippy findings from the review are also cleared;
  remaining clippy work is outside the provider slice.
- [x] promote one app-owned live task contract for cancellation and progress streaming
  Completed: `core::app::AppLiveTask` is now the public live-task input contract. Stable and
  extended app services expose `*_live` methods for long-running operations, while existing
  collected `TaskRun<T>` methods remain convenience wrappers. GUI callers can now stream
  `TaskProgressEvent` payloads and provide cancellation without reaching into internal service
  callback methods or `core::task` runners.
- [x] turn clippy into an actionable refactor gate
  Completed: `cargo clippy --all-targets -- -D warnings` now passes without blanket allows. The
  cleanup removed mechanical warnings and converted the meaningful long-argument pressure into
  context/request objects for dependency collection, package preparation, lock-source preparation,
  addon mutation execution, bundle plan assembly, index attach result creation, and task byte
  progress payloads.

Exit criteria:

- addon update failures are transaction-complete across primary packages and dependency packages
- ignored policy is enforced before expensive work in the common index-update path
- config sync has a product-owned app contract
- provider internals are split enough that cache and materialization work have clear ownership
- frontend callers can observe and cancel long-running tasks without internal service seams
- clippy can run as a useful guardrail during subsequent fearless refactors
