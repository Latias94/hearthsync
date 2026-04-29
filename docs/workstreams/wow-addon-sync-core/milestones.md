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

Completed on 2026-04-21

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
- addon-lock CLI output now shares formatter helpers in `cli::output`, keeping command handlers
  focused on app orchestration while repeated diff/verify/apply text rendering logic lives at the
  presentation edge
- raw `StableAppServices` service accessors and direct runtime access now stay inside the
  `core::app` module boundary, so the public stable boundary stays centered on direct/task
  entrypoints instead of leaking a second service-factory-style API
- `ExtendedAppServices` now composes `StableAppServices` through an explicit stable bridge instead of
  `Deref` compatibility, so the app root no longer masquerades as the stable surface by accident
- the broader non-stable app root is now named `ExtendedAppServices`, which makes its role as an
  extension boundary clearer than the previous `HearthSyncApp` name
- raw `runtime()` access on individual app services is now test-only, so runtime wiring is kept as
  an internal assembly detail instead of a public extension seam
- internal `*Service` implementation types are now app-internal re-exports, so public consumers are
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
- stable and extended app roots now also own constructed service instances directly instead of
  rebuilding new service wrappers from cloned runtime state on every accessor call, so the public
  app boundary behaves like a long-lived service contract rather than a service-factory façade

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
- addon local-archive inputs now follow the same app-boundary rule: CLI records its invocation
  directory as the absolute runtime relative-path base, app addon install/relink requests resolve
  relative local zip sources against that base, and persisted registry sources, addon-lock local
  archive source refs, and explicit addon-lock source overrides must already be absolute before
  core planning or materialization
- addon-index file inputs also resolve at the app boundary now: relative index file paths are
  joined to the runtime relative-path base before inspect, validate, suggest, scaffold, attach,
  install, update, or relink enters the addon-index core, while index-internal local archives stay
  index-relative
- bundle archive inputs and external/config package sources now use the same app-boundary rule:
  relative bundle zip paths, external-package source paths, config source paths, and bundle
  manifest base directories resolve against the runtime relative-path base before stable app
  services enter bundle or external-package core code; bundle output paths keep their existing
  manifest/installation-derived base semantics
- addon-lock and backup selection paths now also resolve at the app boundary: relative lock diff
  files, verify/plan/apply lock files, explicit addon-lock source override archives, backup list
  directories, restore directories, and restore archive paths resolve against the runtime
  relative-path base before addon-lock or backup core code reads them
- app-level output choices now use the same boundary rule: mutation backup output directories,
  backup creation output directories, addon adoption archive outputs, and external-package bundle
  output directories resolve against the runtime relative-path base before core services write
  files. Bundle pack output paths now also resolve before core projection while preserving their
  manifest/installation-derived bundle-domain placement rule.
- installation path inputs now also resolve at the app boundary: relative inspect/resolve
  installation paths and configured installation scan roots are joined to the runtime
  relative-path base before install-discovery core code probes the filesystem.
- CLI-only sidecar files now also resolve through the runtime base before CLI loaders read them:
  bundle pack manifests, manifest validation files, and apply mapping files no longer reintroduce
  process-cwd filesystem reads outside app services.
- addon cache runtime paths are now absolute after boundary handling: CLI one-shot cache
  overrides resolve against the invocation runtime base, `settings set --addon-cache-dir` persists
  resolved absolute paths, and relative persisted cache paths fail closed.
- runtime construction now has a fallible `AppRuntimeBuilder` for path-bearing production
  assembly, so addon provider options, scan roots, and default output directories normalize after
  the relative-path base is known instead of depending on constructor call order.
- runtime policy is now immutable after build: post-build `AppRuntime` mutators were removed, so
  CLI, future GUI code, and tests all use the fallible builder path for scan roots, runtime base,
  default output directories, host platform, addon-state storage, and helper policy.
- resolved installation DTOs now validate before core projection: app services reject relative
  installation tree paths before addon, bundle, backup, config, or policy code can plan reads or
  writes against ambient cwd.
- addon provider options now validate before runtime provider construction: zero retry attempts and
  zero-second HTTP no-validator reuse windows are rejected at the app/runtime boundary instead of
  being silently or ambiguously interpreted inside provider execution.
- runtime settings files now validate the same provider policy before inspect or persistence:
  persisted addon cache directories must be absolute, and zero-second HTTP no-validator reuse
  windows fail before future GUI settings screens can save them.
- bundle manifests now validate at all manifest-read and app boundaries: disk-loaded manifest
  files fail inside `load_manifest()`, embedded bundle manifests fail inside the archive reader,
  and app-owned pack manifests fail before core request construction.
- bundle apply mappings now validate explicit account, server, and character identity segments at
  the app boundary before bundle, external-package, or config apply planning can turn them into
  WTF target paths.
- bundle apply mappings now also have a domain-level validation contract: mapping files, app DTOs,
  and direct planning inputs reject duplicate selected accounts and overlapping character override
  rows before planning.
- bundle manifest validation now also covers portable resource identifiers: addon directory names,
  WTF character identity segments, interface asset root names, and non-empty author metadata fail
  before pack/apply code can turn them into live paths.
- addon package metadata values now reject blank optional text fields and blank supported-flavor
  entries before install requests can persist them into managed state.
- addon-index directory hints now validate as portable, case-insensitively unique addon directory
  segments, so exact `addon_directories` hints cannot be blank, path-shaped, Windows-reserved, or
  ambiguous on Windows/default-macOS targets.
- addon-index package sources and optional source metadata now reject empty local paths, invalid
  HTTP archive URL schemes, zero CurseForge ids, blank GitHub owner/repo/tag/asset fields, and
  blank source metadata before provider materialization or registry writes can trust them.
- tracked addon registry load/save now reuses the same source-reference shape validation, keeps the
  stricter absolute-local-archive state rule, validates addon directory names as portable segments,
  and rejects blank stored metadata fields or supported-flavor entries.
- addon-lock files now fail closed at read time on invalid source refs, relative local archive
  refs, blank package metadata/timestamps, invalid content hashes, non-portable addon directory
  names, and case-insensitive addon-directory ownership conflicts; bundle addon-lock embedding now
  reuses the same validated loader and validates generated locks before persistence.
- addon-lock source sidecars now reject empty source lists, blank or whitespace-padded comparison
  keys, non-portable source paths, and case-insensitive duplicate source paths before source
  override resolution.
- embedded bundle addon source indexes now fail closed before source extraction on blank or
  duplicate comparison keys, invalid source paths, invalid content hashes, and non-portable addon
  directory declarations.
- addon-policy files now also fail closed on blank update timestamps, blank or duplicate package
  ids, no-op package rows, empty version pins, and zero file-id pins, with the same validation
  applied before writes.
- installation path normalization now preserves Windows verbatim UNC roots when trimming `\\?\`
  prefixes, so network-share installs remain absolute UNC paths after canonicalization cleanup
- external-package source analysis now rejects directory and zip symlink entries explicitly instead
  of treating symlink targets as portable regular-file payloads
- archive path validation now rejects Windows-reserved segment characters, device names, and
  trailing-dot / trailing-space segments so cross-platform imports fail early
- backup restore archive preparation now rejects symlink entries up front and reuses the same
  portable archive-segment validation as bundle/external-package ingest, so restore-time path
  safety no longer trails import-time path safety
- backup metadata now validates at the archive metadata boundary: `backup.toml` symlink/directory
  entries, invalid schema/timestamps/flavor/group rows, and blank labels fail before catalog or
  restore-selection code consumes metadata.
- backup labels now normalize before archive creation and must be portable filename segments both
  when creating new backups and when reading stored backup metadata.
- backup creation now also rejects local directory/interface symlink entries instead of following
  link targets into the archive, keeping backup payloads bounded to the intended WoW tree
- addon local-archive package preparation now also rejects zip symlink entries, so addon
  install/update flows share the same archive-metadata safety floor as backup restore and
  external-package ingest
- addon local-archive preparation now also validates staged addon-directory and file target
  collisions using the selected target platform's case-sensitivity rules, so Windows/default-macOS
  installs fail fast on case-only archive conflicts instead of depending on host filesystem behavior
- cross-platform target-path collision detection now also shares one canonical helper in
  `core::archive_path`, and backup restore reuses the same case-folding rules as addon prep,
  bundle planning, and external-package normalization instead of carrying its own duplicate logic
- the same shared path-safety layer now also detects file/directory ancestor conflicts with
  platform-aware case folding, so addon prep, bundle planning, backup restore, and
  external-package normalization all reject target hierarchies that would only collide on
  Windows/default-macOS filesystems
- zip-style archive path serialization now also lives in the shared `core::archive_path` layer,
  so backup zip writing and bundle zip writing no longer keep parallel slash-normalization helpers
- zip-writing entry creation now also reuses shared portable-path validation in
  `core::archive_io`, so backup creation and bundle packing reject non-portable archive names
  before emitting first-party zip payloads
- first-party backup/bundle export flows now also maintain one shared archive output path set
  while writing, so case-only duplicate names and file/directory hierarchy conflicts fail before
  any non-portable first-party zip layout is emitted
- first-party backup and bundle export output tracking now also has focused pure-logic regression
  coverage for case-only metadata collisions, file-as-ancestor conflicts, and legal directory
  ancestors, including bundle addon sidecar metadata/source paths
- first-party backup and bundle export now also share one archive-output issue-to-error mapping
  helper in `core::archive_io`, so duplicate collision/prefix-conflict wording no longer lives in
  parallel wrapper functions
- unsupported-symlink rejection now also shares one canonical helper in `core::archive_io`, so
  addon archive prep, backup restore/source scanning, bundle archive ingest, and
  external-package ingest no longer keep separate identical error branches
- external-package directory sources now reuse the same portable segment validation as zip
  sources, so non-portable relative names fail consistently before normalization
- external-package normalized-path collision rules now also have pure logical regression coverage
  in addition to zip-fixture coverage, so Windows/default-macOS case-folding behavior no longer
  depends only on integration fixtures that a host filesystem may be unable to materialize
- author-package zip and directory analysis now also explicitly covers common macOS resource-fork
  metadata and desktop noise, keeping `__MACOSX`, `._*`, `.DS_Store`, `Thumbs.db`, and
  `desktop.ini` artifacts out of normalized package entries and warning counts
- archive-compatibility coverage now also includes a larger wrapped author zip with a dozen addon
  roots, more than one hundred normalized files, WTF/fonts/interface resources, and repeated
  archive noise, so resource summary behavior is no longer only covered by tiny fixtures
- bundle archive inspect/apply/addon-lock extraction now also reject symlink entries up front, so
  first-party bundle ingestion no longer trails addon/external-package/backup archive safety
- bundle archive entry validation now also rejects non-portable path segments during inspect/apply,
  aligning first-party bundle archives with the same portable-name floor as other archive inputs
- bundle plain-name validation now also reuses the shared portable single-segment rules, so addon
  names, account/server/character mapping names, interface asset names, and addon-index file names
  reject Windows-reserved names and trailing-dot / trailing-space segments before bundle
  planning/export depends on host filesystem quirks
- addon lock sidecar source-index paths now also reuse the shared zip-segment validation floor, so
  malformed `sources.toml` entries fail consistently before sidecar-relative source resolution
- addon lock sidecar source-index resolution and bundle embedded addon-source index extraction now
  also share one rooted `sources/` path helper under `core::archive_path`, while preserving
  caller-specific error context instead of keeping parallel root checks
- zip archive entry validation now also shares one helper in `core::archive_io` for symlink
  rejection plus non-directory portable-path validation, and addon archive prep /
  external-package zip ingest now also reuse the same helper to return owned file segments instead
  of validating and reparsing the same entry name separately
- bundle addon-source archive names and embedded addon-index metadata file names now also use
  case-insensitive uniqueness checks before zip output, so Windows/default-macOS bundles fail or
  suffix deterministically without relying on late archive-writer collision detection
- addon-index package IDs are now also validated with the same case-insensitive semantics used by
  index package lookup, so curated indexes cannot define ambiguous IDs such as `Details` and
  `details`
- addon registry load/save now also validates schema version, non-empty package IDs and
  addon-directory sets, case-insensitive package IDs, and case-insensitive addon-directory
  ownership, while addon-index bulk matching records used tracked IDs with the same normalized key
  so curator attach/update flows do not depend on case-sensitive registry assumptions
- addon install planning and addon mutation execution now also resolve existing live AddOns
  entries through the selected installation platform's case-folding rules, so Windows/default-macOS
  dry-runs catch `Details` vs `details` conflicts early and replace/update/remove paths remove the
  actual on-disk entry even when archive casing differs
- addon-lock planning and verification now use the same platform-aware addon-directory keys for
  tracked owners, freed directories, untracked replace checks, and missing-directory detection, so
  lock plans preserve Linux case-distinct behavior while Windows/default-macOS targets treat
  case-only live AddOns as the same directory
- external-package staging materialization now also reuses the shared
  `core::bundle::entry_layout` classifier for normalized bundle-style paths, so addon/WTF/fonts/
  interface root dispatch no longer lives in a second parallel match tree when building a
  first-party bundle from an author package
- normalized external-package entries now also derive `group`, `wtf_scope`, and source
  account/server/character identity from the shared `core::bundle::entry_layout` classifier, so
  author-package analysis no longer maintains a second semantic mapping from normalized bundle
  paths to apply-group and WTF-scope meaning
- external-package warning taxonomy no longer carries the unreachable
  `unsupported_wtf_root_savedvariables` code; supported root saved-variable entries now normalize
  into bundle layout directly, and the remaining warning codes map only to still-reachable
  unsupported WTF layouts
- external-package WTF source classification now also splits pure suffix-to-layout recognition
  from warning rendering and entry assembly, so future author-package WTF layout changes can be
  tested and evolved at the semantic layer without coupling every branch to source-path-specific
  warning text generation
- external-package non-WTF rooted path handling now also splits pure
  fonts/interface/AddOns subtree recognition from entry assembly, so future author-package
  resource-root changes can be tested as path semantics without coupling every branch to
  `ExternalPackageEntry` creation or warning-object construction
- external-package addon classification now also splits pure addon-root / missing-root semantics
  from entry assembly and warning creation, so `.toc`-driven root discovery, addon path
  normalization, and addon-subtree warning behavior can evolve independently within one
  source-to-normalized-path pipeline
- backup restore archive preparation now also parses each entry name into one shared
  `group + destination` result instead of reparsing the same zip path separately for group
  membership and target-path resolution, while preserving the distinction between unsupported root
  entries and root-only unsupported entry paths
- backup archive root naming now also lives with `BackupGroup`, so backup group metadata labels
  (`interface_assets`) stay distinct from archive root names (`interface`) while creation-time
  root emission and restore-time root parsing both reuse the same group-owned contract instead of
  keeping parallel hardcoded root strings
- bundle apply planning now rejects target-path collisions using the selected target platform's
  case-sensitivity rules, with regression coverage for macOS rejection and Linux case-distinct
  planning
- addon-root discovery and prefix matching now also follow platform-aware case rules, so
  Windows/default-macOS author-package normalization and addon-archive install/update flows no
  longer silently drop mixed-case addon subtree files while Linux still preserves case-sensitive
  distinct-root behavior
- Windows-to-macOS author-package apply coverage now also includes a complex zip with a wrapper
  directory, mixed-case `Interface/AddOns` subtree paths, WTF/fonts/interface resources, default
  author-package policies, backup creation, and macOS/desktop noise that must not be imported
- the next `M4` slices should now focus on archive compatibility, Windows-to-macOS regression
  coverage, and remaining path-portability hardening

## M5 - Update Correctness and Transaction Completion

### Status

Completed on 2026-04-21

### Goal

Fix the remaining correctness gaps that would otherwise make addon update, addon-lock sync, and
frontend-facing preview semantics unreliable in real-world repeated use.

### Deliverables

- explicit freshness rules for mutable remote addon sources versus immutable pinned artifacts
- provider caching behavior that does not silently freeze mutable remote references
- addon-lock apply semantics that remain coherent across execute, verify, and cancellation phases
- a documented decision on whether default public plan is purely logical or intentionally includes
  expensive compare/rewrite-aware preview behavior
- hardened remote archive naming rules for portable Windows/macOS inputs

### Exit Criteria

- repeated `addon update` runs can fetch newer artifacts for mutable sources instead of always
  reusing a stale cached download
- addon-lock apply either succeeds as one verified operation or fails with a deterministic
  rollback/outcome contract
- bundle and external-package plan behavior is explicit enough that future GUI dry-run work does
  not depend on accidental planner cost
- remote download naming rules remain portable across supported host platforms

### Current Notes

- this milestone intentionally stays inside `wow-addon-sync-core`; it should not become a parallel
  workstream because the findings cut across existing provider, planning, mutation, and app
  boundaries
- the review that motivates this milestone is recorded in `review-2026-04-21.md`
- the current codebase already has strong regression coverage and a cleaner planner/app boundary
  than earlier revisions, so this milestone is about closing high-impact correctness gaps rather
  than reopening the entire architecture
- first progress is now in place at the provider layer: mutable remote references no longer reuse
  cached archives blindly, floating GitHub/CurseForge sources resolve to concrete artifacts before
  cache reuse, and URL-derived archive naming no longer treats query strings or fragments as part
  of the file name
- addon-lock apply now also rolls back when verification cannot complete after execution because of
  cancellation or verification-time errors, so callers no longer receive a failure while the local
  AddOns state has already been left mutated
- successful addon-lock apply still returns the verification report itself, which preserves the
  distinction between “verification completed and found drift/untracked addons” versus
  “verification could not complete and the operation was rolled back”
- default public planning is now explicitly logical and conservative: `bundle plan` and
  `external-package plan` no longer read source bytes or run rewrite-aware target comparison just
  to refine existing-target actions into exact skip/replace results, while prepare/apply still
  keeps that exact identical-file detection before execution
- addon registry and addon-lock writes now also use same-directory temporary files plus atomic
  replacement, so `addons.toml` and `lock.toml` no longer rely on direct in-place overwrite during
  normal state persistence

## M6 - Structured Task Progress for Frontends

### Status

Completed on 2026-04-21

### Goal

Upgrade the stable task contract from phase-plus-message progress into a task identity plus
structured progress model that future `egui` work can consume directly.

### Deliverables

- generated `task_id` on `TaskRun` plus matching task ids on collected-progress and callback events
- optional machine-readable progress fields on `TaskProgressEvent`
- shared task helpers for phase, step, and future byte-oriented progress emission
- structured step progress coverage for key execution loops
- regression tests at the task layer and app boundary

### Exit Criteria

- frontend callers can correlate one logical task without deriving identity from text messages
- step-oriented work can report deterministic `current/total` counts through one shared event shape
- the stable task contract keeps CLI-readable messages while becoming GUI-friendly enough for reuse

### Current Notes

- this milestone stays inside `wow-addon-sync-core`; it is a reusable core-contract correction, not
  a separate frontend workstream
- `TaskProgressEvent` now keeps `message` for humans while also exposing optional `code`,
  `current`, `total`, `bytes_current`, `bytes_total`, and `bytes_per_second`
- `core::task` now generates task ids inside the shared wrapper layer, so business operations do
  not each invent their own identity scheme
- addon directory mutation, backup restore execution, metadata-only addon-lock actions, and bundle
  apply operations now emit typed step progress instead of only text
- provider-backed addon archive downloads now also emit `TaskProgressCode::DownloadArchive` with
  `bytes_current`, `bytes_total`, and `bytes_per_second` during package preparation for addon
  install/update, addon-index install/update, and addon-lock source resolution
- validation for the current follow-up slice passed with `cargo fmt` plus `cargo nextest run`
  (`386/386` passing)

## M7 - Config Sync Ergonomics Before Desktop UI

### Status

Active

### Goal

Keep the current reusable core and planning model, but remove the remaining product and safety gaps
that would otherwise force `egui` to invent its own config-sync semantics or addon policy state.

### Deliverables

- placeholder-safe text Lua rewriting aligned with the byte rewrite path
- a first-class config-oriented command/app surface layered on the existing planning engine
- a separate addon policy/preferences model distinct from the reproducible addon lock
- integrity-aware cached download reuse with operator-facing cache semantics
- sanitized real-world SavedVariables regression coverage

### Exit Criteria

- configuration sync is a first-class product surface without introducing a second config engine
- addon policy and addon reproducibility are modeled as separate concerns
- cached immutable downloads have explicit integrity behavior instead of only presence-based reuse
- rewrite confidence for localized SavedVariables is backed by real regression fixtures

### Current Notes

- this milestone still belongs to `wow-addon-sync-core`; it should not become a separate workstream
  because the remaining issues cut across provider acquisition, Lua rewrite safety, addon policy,
  and frontend-facing product vocabulary at the same time
- the current codebase is already much stronger than the older NewBeeBox gap notes: root WTF cache
  files, `AddOns.txt`, mutable-source freshness, and byte-level task progress are now part of the
  implemented core instead of remaining open design wishes
- the remaining work is now concentrated in four places:
  - internal task-execution consolidation below the stable app boundary
  - generic transport-level cache validator coverage
  - remaining addon policy execution gaps
  - sanitized real-world SavedVariables coverage
- first progress inside this milestone is now in place: the text Lua rewrite path no longer uses
  collision-prone fixed placeholders, and dedicated regression coverage now verifies that existing
  `__HEARTHSYNC_REWRITE_<n>__` literals in user content survive rewriting unchanged
- second progress inside this milestone is now in place: the CLI now exposes `config inspect`,
  `config plan`, and `config apply` as a first-class product entrypoint while still reusing the
  existing external-package planning/apply engine underneath
- the stable app boundary now also exposes first-class config-oriented request and task entrypoints
  on top of the same engine, so future GUI work no longer has to enter config sync only through
  external-package-shaped app contracts
- the first stable-surface shrink pass is now also in place: long-running `StableAppServices` and
  `ExtendedAppServices` entrypoints return `TaskRun<T>` directly, while CLI convenience helpers
  now render `run.result` instead of forcing the app boundary to keep direct,
  `*_collecting_progress`, and `*_with_callbacks` triplets for the same operation
- that means the remaining task-surface gap is now narrower and internal: callback streaming still
  exists below the stable boundary for lower-level reuse and tests, but future public streaming
  work should return through one explicit task-context abstraction rather than reintroducing
  per-operation triplets
- addon policy now also has a first concrete product/state seam: mutable preferences persist in
  the managed addon state backend (default platform app-data; sidecar portable mode optional), the
  stable app boundary exposes
  inspect/set/remove entrypoints, and the CLI now has a matching `addon policy` namespace without
  weakening `lock.toml`'s reproducible contract
- addon update now also has a first bounded policy-execution slice: bulk `addon update` and bulk
  `addon index update` honor `ignored = true`, explicit named addon update can override `ignored`,
  and provider-backed addon update applies basic pin overrides (`pin.file_id` for CurseForge,
  `pin.version` as a GitHub tag override) while preserving the tracked `package_id`
- regular provider-backed addon update now also forwards `release_channel` and
  `allow_prerelease` into provider resolution while preserving tracked source identity:
  floating GitHub releases can opt into prerelease selection, floating CurseForge mods can filter
  by stable/beta/alpha release type, and explicit GitHub tag / CurseForge file-id pins remain
  authoritative instead of being re-filtered as floating references
- regular provider-backed addon update now also consumes `install_dependencies = true` for
  supported sources by installing missing required CurseForge dependencies as additional tracked
  packages inside the same update transaction; unsupported source kinds now fail explicitly instead
  of silently treating the field as a no-op
- addon-index update now also consumes `install_dependencies = true` for the same first
  dependency-installation slice, but it resolves missing required dependencies from the curated
  index source while keeping that source authoritative instead of letting mutable policy rewrite it
- addon-policy execution now also has an explicit code-level split between provider-backed update
  policy and indexed-update policy, so pin and release-channel/prerelease overrides are
  structurally unavailable to `addon index update` instead of only being filtered by convention
- provider-side dependency resolution now also uses an explicit strategy result instead of a bare
  dependency-source list, so the current `missing required only` slice is encoded in the contract
  that regular update and addon-index update both consume
- provider-side dependency support now also has an explicit capability contract, so unsupported
  sources and `missing required only` sources are distinguished before execution instead of only by
  downstream validation strings
- that dependency capability now also projects through app-owned source values, so stable app
  callers can inspect dependency support from addon inventory/search/index/lock results without
  coupling themselves to provider-domain types or waiting for an update failure
- addon and addon-index app services now also preflight `install_dependencies = true` against
  provider dependency capability before they enter domain update execution, so unsupported sources
  fail at the app boundary instead of only surfacing from prepare-stage validation
- addon-index matching now also uses provider-level source-family identity in both preflight and
  domain update flows, so index-package id drift or GitHub asset-name drift no longer forces a
  fallback when the tracked source still identifies the same underlying package family
- addon-index matching now also accepts unique exact display-name continuity as a later fallback,
  so source-family migration can still preflight when the curated package name remains stable
  across tracked package id, stored metadata package name, addon directory name, or addon title
- addon-index schema now also supports exact author-declared `match_package_ids` hints, and both
  preflight and domain matching reuse them before falling back to weaker continuity signals
- the remaining addon-policy gap is now narrower still: indexed update still does not consume pin
  or release-channel/prerelease source-resolution overrides, and dependency execution is not yet a
  generic cross-provider or dependency-upgrade policy
- addon-index preflight matching is still intentionally conservative only when the curated source
  family itself changes, the index package omits exact `match_package_ids`, stable
  `addon_directories`, and exact unique display-name continuity, and there is no stronger stored or
  source-based identity either; that narrower case still falls back to the domain validation path
- that remaining fallback path now also emits explicit guidance when unsupported dependency-install
  policy is only discovered during domain preparation, so operators and curators can see that app
  preflight lacked stable identity hints and which exact hint types would close the gap
- `addon index inspect` now also reports structured exact-identity-hint coverage, so CLI and
  future GUI surfaces can flag packages that still lack any exact migration bridge instead of only
  echoing raw package fields
- that inspect surface now also emits structured warning objects with explicit blocking/advisory
  severity, so downstream consumers can bind on stable warning codes instead of scraping human
  text from summary lines
- those warning codes now distinguish the hard "missing both exact hints" case from softer
  curation follow-up: `missing_exact_identity_hints` is blocking, while
  `missing_match_package_ids` and `missing_addon_directories` remain advisory
- `addon index validate` now also exists as an explicit curator gate, returning a structured
  validation result while making the CLI fail fast only on blocking addon-index curation warnings
- `addon index suggest` now also exists as the first explicit curator authoring helper: it reuses
  local tracked-registry state plus the existing preflight matching order to suggest missing
  `match_package_ids` and `addon_directories` additions, while still reporting no-match and
  ambiguous-match cases as structured outcomes instead of inventing new runtime matching rules
- that suggestion path now also resolves local-archive index sources through the same
  index-relative canonicalization used by install/update, so local curated-package authoring no
  longer loses source-identity matches just because the registry stores canonicalized archive paths
- `addon index scaffold` now also exists as the first bootstrap authoring path: it writes a new
  curated index from the current tracked addon registry, preserves curated metadata when already
  present, infers only the remaining package name/version fields from tracked addon state, and
  refuses to overwrite an existing index unless explicitly told to do so
- that scaffold path now also fails closed when no tracked addon registry exists yet, so an
  untracked local addon folder scan cannot silently turn into a fake curated source inventory
- cache integrity now also has a first concrete floor: immutable-source cache reuse writes a local
  integrity sidecar and re-hashes cached archives before reuse, so missing metadata or locally
  modified cached files trigger a refresh instead of being trusted because the file merely exists
- provider-backed immutable cache reuse now also consumes first remote validators where the source
  API already exposes them: GitHub release assets and CurseForge files refresh the cached archive
  when published size, modified time, or digest/hash metadata changes even if the cached file still
  matches the previous local sidecar
- generic `http(s)` archives now also use one shared transport-level conditional GET path through
  the provider HTTP layer: when a cached archive already matches the local integrity sidecar and a
  server exposes stable `ETag` or `Last-Modified` headers, HearthSync now sends
  `If-None-Match` / `If-Modified-Since` and reuses the cache on `304 Not Modified` instead of
  refreshing unconditionally
- URLs without reusable transport validators now also have an explicit bounded fallback instead of
  refresh-on-every-run: cache sidecars record fetch time, the default provider policy reuses
  no-validator cache entries only within a short freshness window, and legacy sidecars without a
  fetch timestamp fail closed by refreshing
- provider, stable app, and CLI now also expose first operator-facing cache maintenance flows:
  `addon cache purge` clears the configured download cache root, while `addon cache repair`
  removes incomplete downloads, orphaned archives, and invalid sidecar-backed cache entries
- `addon cache repair` now also performs first remote validator-driven maintenance: HTTP cache
  entries with reusable transport validators use conditional GET during repair, resolved
  GitHub/CurseForge entries compare fresh provider metadata and refresh when validator state
  changes, and expired no-validator HTTP entries are pruned
- provider cache semantics are now also operator-facing instead of only runtime-internal:
  global CLI/runtime options can configure the cache directory plus the no-validator HTTP
  freshness policy, and runtime diagnostics project that configured policy through the stable app
  capability surface
- cache and addon-state runtime defaults now also have one explicit persistent settings backend:
  `core::app` persists selected runtime overrides under app-data `settings/runtime.toml`, the CLI
  exposes `settings inspect|set|reset`, and one-shot CLI flags override persisted values instead
  of duplicating persistence logic
- the remaining cache-validity gap is now narrower still: remote repair is best-effort rather
  than a richer operator-configurable policy surface, and GUI work still needs a richer settings
  surface on top of the shared backend
- SavedVariables rewrite safety now also has a first sanitized real-world fixture floor: the
  `lua_patch` tests include a more realistic UTF-8 `Details.lua` sample with Chinese text plus a
  more realistic Latin-1 `Pawn.lua` sample with extended characters, so localized text
  preservation and non-UTF-8 identity rewrites are no longer only validated through tiny synthetic
  snippets
- fixture breadth now also covers a more realistic UTF-8 `Clique.lua` slice with Chinese
  character/server profile keys plus a more realistic UTF-8 `BagSync.lua` slice with
  realm-and-character keyed account data, so the rewrite floor is no longer limited to one
  profile-key addon shape and one Latin-1 character-root addon shape
- fixture breadth now also covers a more realistic UTF-8 `AddOnSkins.lua` slice with `profiles`
  plus `profileKeys` keyed by `角色 - 服务器`, so the positive floor now also includes a
  profile-key-driven addon that does not rely on the broader identity whitelist
- fixture breadth now also covers a more realistic UTF-8 `ElvUI.lua` slice with mixed
  `角色 - 服务器` profile keys, nested realm/character maps, and no-space `角色-服务器`
  combined identity keys, so rewrite coverage now includes the current support for both spaced and
  compact combined identities
- fixture breadth now also covers a more realistic UTF-8 `NewBeeBox.lua` slice with no-space
  `服务器-角色` combined identity keys plus separate `name` / `realmName` fields, so rewrite
  coverage now also includes the current narrow support for reversed compact combined identities
  without touching `Player-...` GUID fields
- identity rewrite targeting is now narrower too: unknown `SavedVariables/*.lua` files no longer
  enter identity rewrite only because they contain generic `playerName` / `realm`-style fields,
  so real multi-character account files such as `Syndicator.lua` now fail closed unless HearthSync
  has an explicit known-file rule for them
- that fail-closed boundary is now also explicit for `WIM.lua`-style chat history and cache
  payloads: nested `服务器 -> 角色 -> 会话历史` data is regression-covered as out of scope for
  automatic config rewrite rather than silently expanding the supported rewrite surface
- `Rarity.lua` is now narrower too: profile keys still rewrite through the generic
  `profileKeys` signal path, but account-wide statistics no longer ride the identity-rewrite
  whitelist because real payloads contain many same-server characters and broad `playerName` /
  `server` replacement would mis-target unrelated records
- fixture breadth now also covers a more realistic UTF-8 `RurutiaSuite.lua` slice plus a more
  realistic UTF-8 `NDui_Bags.lua` slice with a long single-line `profileKeys` payload, so the
  generic profile-key path is now regression-covered against author text, mixed
  simplified/traditional character variants, legacy suffixed profile keys, and dense one-line
  account maps without widening the identity whitelist
- the UTF-8 `profileKeys` path is now narrower too: profile-style rewrites are scoped to direct
  `profileKeys` entries, direct `profiles` keys, and `*profileKey` field values instead of whole-
  document exact-string replacement, and `Clique.lua` now also rides an explicit known-file
  identity rule because its real payload keeps `char` tables keyed by `角色 - 服务器`
- the UTF-8 known-file identity path is now narrower too: explicit identity field values such as
  `playerName`, `realm`, `server`, `character`, `LastPlayerFullName`, `LastRealm`, `guildrealm`,
  `realmKey`, `rwsKey`, and paired `name + realmName` no longer rely on whole-document quoted-
  string replacement, so real `Details.lua`-style `lastPlayerName` text now stays untouched while
  real `NewBeeBox.lua` player records still rewrite correctly
- the UTF-8 identity-key path is narrower again: exact identity-shaped Lua keys now rewrite only in
  known containers such as root table-key records, `profileKeys`, `profiles`, `char`, `faction`,
  `worldBoss`, `searchHistoryList`, `Toons`, `value`, `currentrealm`, and `totals`, plus
  root/faction `服务器 -> 角色` maps, so arbitrary nested cache keys that merely equal
  `角色 - 服务器`, `角色-服务器`, or `服务器-角色` now stay untouched
- the non-UTF-8 byte fallback is now also scoped by Lua structure instead of whole-document text
  replacement, and UTF-8 payloads no longer fall through into byte rewriting after a scoped
  rewrite miss
- fixture breadth now also covers BigWigs-style profile keys without broad note rewriting plus a
  Baganator-style recent-character cache that remains fail-closed instead of treating history
  strings as supported identity data
- the main remaining safety gap is now narrower: new addon-specific identity-key containers still
  require explicit evidence before they should join the shared allowlist, and fixture breadth is
  still limited
- the remaining rewrite-fixture gap is now breadth rather than total absence: more addon-specific
  SavedVariables shapes and more encoding/pathology variants are still needed before broad desktop
  migration claims feel justified
- the bounded review that motivates this milestone is recorded in
  `review-2026-04-21-config-sync-gap.md`

## M8 - Tracked Registry Bootstrap for Existing Local Installs

### Status

Active

### Goal

Let existing manual addon installs enter tracked HearthSync state without inventing fake remote
source identity or requiring operators to hand-build the registry first.

### Deliverables

- explicit `addon adopt` bootstrap flow for untracked local addon directories
- tracked-registry bootstrap semantics documented in the core workstream instead of a new parallel
  workstream
- local snapshot archive creation that reuses the existing archive/source model
- minimal `addon relink` flow that upgrades adopted snapshot state to a real source without
  reinstalling addon files
- curator-aware `addon index relink` flow that can attach curated metadata without reinstalling
  addon files
- guided bulk `addon index attach` flow that batches curator-aware relinks without reinstalling
  addon files
- regression coverage for explicit adopt safety and follow-up curator bootstrap flows

### Exit Criteria

- a machine with only manual addon directories can create a real tracked registry without scanning
  the whole `Interface/AddOns` tree implicitly
- adopted packages are represented by real local snapshot archives rather than fake remote sources
  or self-referential local-directory sources
- operators can later switch one tracked package from an adopted snapshot to a real source while
  keeping the live installation untouched, as long as addon-directory identity matches exactly
- operators can also attach one tracked package to a curated index package, including metadata-only
  attach when the source already matches, without forcing reinstall
- operators can preview and batch-attach multiple tracked packages against one curated index, with
  default all-or-nothing registry writes and an explicit reviewed ready-only partial apply option
- `addon index scaffold` and `addon index suggest` become usable immediately after explicit adopt

### Current Notes

- this milestone intentionally stays inside `wow-addon-sync-core`; it is a product-core boundary
  correction, not a new workstream
- curator scaffolding is now strong once tracked registry state exists, but the product still
  needed a safe answer for the common "manual install first, curate later" machine state
- the chosen direction is explicit adoption rather than ambient discovery: operators select the
  untracked addon directories they want to group, optionally provide a tracked `package_id`, and
  HearthSync snapshots those directories into a real local archive under managed addon state
  storage by default
- that keeps the current source model honest: tracked packages continue to point at a real archive
  source instead of pretending to know a remote provider or pointing update semantics back at the
  mutable live installation itself
- runtime now owns addon-state backend selection too: managed addon registry, lock, policy, and
  adopted snapshot archives default to platform app-data keyed by installation identity, while
  sidecar `.hearthsync` remains an explicit portable backend rather than the desktop default
- that also makes the user-visible filesystem behavior clearer: default scan-only flows and default
  managed-addon flows no longer need to create `Interface/AddOns/.hearthsync`; that path now
  signals either explicit portable sidecar mode or explicit bundle sidecar metadata output
- operators now also have a first explicit product control for that backend choice: the CLI accepts
  global `--addon-state-storage app-data|sidecar`, so backend selection stays centralized at the
  runtime layer instead of leaking into per-feature path flags
- bundle export and bundle addon-lock shortcut flows now also respect that runtime backend when
  they access active tracked addon state; only unpacked bundle metadata continues to use the
  explicit `.hearthsync/bundles/<bundle-id>/` sidecar location
- the stable app capability surface now also projects addon-management semantics explicitly:
  frontend callers can read the selected addon-state backend and the `scan-only` versus
  `managed mode` contract from app-owned runtime capability values instead of scraping CLI help or
  reverse-engineering documentation
- the product now also has a first explicit runtime-diagnostics query path: CLI and future GUI
  callers can read host platform, scan roots, default output dirs, addon backend, provider mode,
  and helper strategy from one stable app-owned surface
- that runtime-diagnostics surface now also projects exact managed addon state paths when an
  installation context is supplied, so operators and future GUI settings screens do not need to
  reimplement app-data versus sidecar path resolution on their own
- the canonical app-data root has now also been normalized before the layout becomes stable:
  addon state and default backup storage resolve through one shared application-only
  `ProjectDirs` helper, and the pre-release codebase now keeps only that canonical layout instead
  of carrying a legacy fallback branch
- the first follow-up upgrade path is now explicit too: `addon relink` validates one new source
  against the already-tracked addon-directory set and rewrites only registry source state
- generic relink deliberately clears stored package metadata and requires exact addon-directory
  parity
- curator-aware relink is now explicit too: `addon index relink` rewrites both registry source and
  curated metadata, while still refusing addon-directory mismatches and still leaving live AddOns
  untouched
- the higher-level curator workflow now exists too: `addon index attach` batches that same
  curator-aware relink model across one index, reuses suggestion-style matching order, returns a
  structured review result for ready/blocked/skipped packages, and only writes the registry when
  every selected package is safe to attach
- the default bulk attach path remains all-or-nothing, while explicit ready-only partial apply is
  now available for reviewed operator workflows. Partial apply writes only ready planned changes,
  reports `partial_apply`, and leaves blocked packages visible in the result instead of pretending
  the whole batch was safe.

## M9 - Pre-GUI Architecture Hardening

### Status

Completed on 2026-04-28

### Goal

Close the remaining architecture and behavior risks that would otherwise become accidental
contracts for the future `egui` frontend.

### Deliverables

- rollback-aware addon update execution that covers dependency installation after primary package
  updates
- policy-first bulk index update planning for ignored packages
- config-owned app request/result DTOs that hide external-package internals from product callers
- decomposed addon provider modules for cache, materialization, validation, and source adapters
- one app-owned live task contract for progress and cancellation
- clippy baseline cleanup sufficient for `cargo clippy --all-targets -- -D warnings`

### Exit Criteria

- update execution either fully succeeds or reports a rollback-aware failure after every post-backup
  mutation step
- ignored bulk index packages do not perform provider download or archive preparation in the common
  preflight-match path
- GUI code can consume configuration sync through config-named app contracts
- provider cache and materialization changes have focused module ownership
- live task progress and cancellation are stable app concepts, not internal service details
- clippy can be used as a practical quality gate during later refactors

### Current Notes

- this milestone intentionally stays inside `wow-addon-sync-core`; the 2026-04-28 findings cut
  across shared core boundaries rather than forming a new product workstream
- the motivating review is recorded in
  `review-2026-04-28-architecture-hardening.md`
- because HearthSync is still pre-release, this milestone may delete obsolete transition paths and
  reshape app contracts instead of preserving compatibility for old call sites
- first progress is now in place: regular addon update and addon-index update share one
  dependency-aware mutation helper that saves the addon registry only after primary packages and
  dependency packages all succeed, so dependency install failure after a primary update now rolls
  back through the same outcome path instead of leaving half-updated tracked state
- second progress is also in place: bulk addon-index update now skips preflight-matched ignored
  packages before source resolution or package preparation, so ignored packages do not trigger
  provider download work in the common stable-match path
- third progress is also in place: config sync no longer exposes `external-package` response
  aliases at the app boundary. The config service still delegates to the shared import engine, but
  it now projects concrete config-owned inspection, plan, apply, warning, summary, and source-kind
  DTOs for CLI and future GUI callers.
- fourth progress is also in place: addon provider internals are split into focused modules:
  `cache.rs` owns cache metadata, purge/repair, freshness, and cache-local download utilities;
  `materialize.rs` owns source materialization; `validation.rs` owns remote validator/checksum
  construction; `source.rs` owns source identity helpers; and `source_adapter.rs` owns
  provider-adapter search, dependency, and release-policy glue. `mod.rs` now stays centered on the
  provider trait, request/result contracts, options, retry wrapper, and default-provider
  composition. Provider-local clippy findings were reduced out of the remaining baseline; the
  follow-up clippy milestone is now concentrated in addon mutation/package-prep, task progress,
  bundle/install/lua helpers, and tests.
- fifth progress is also in place: `core::app::AppLiveTask` is the stable live-task input contract,
  and stable/extended services now expose public `*_live` entrypoints for long-running operations.
  These entrypoints reuse the same cancellation polling and `TaskProgressEvent` stream as the
  existing callback runner, while `TaskRun<T>` remains the collected-progress convenience result.
- sixth progress is also complete: `cargo clippy --all-targets -- -D warnings` now passes. The
  cleanup kept mechanical style fixes separate from real boundary fixes, and the meaningful
  long-argument warnings now use request/context payloads instead of broad lint suppression.

## M10 - Pre-GUI Boundary Contract Hardening

### Status

In progress

### Goal

Make every file, app, and remote-provider boundary fail closed before future `egui` code consumes
the resulting DTOs as stable product state.

### Deliverables

- validated readers for manifests, apply mappings, addon locks, backup metadata, and sidecar
  source indexes
- provider response validation for remote archive identities before cache path construction or
  download execution
- regression coverage for cross-platform path identity, duplicate detection, and invalid remote
  metadata

### Exit Criteria

- app and CLI callers do not need parallel validation for loaded files or fetched provider DTOs
- unsafe names, duplicate case-folded identities, blank semantic fields, and invalid download URLs
  fail before planning or execution mutates state
- future GUI forms can project and edit app-owned DTOs without preserving raw parsed-but-invalid
  values

### Current Notes

- recent progress has moved manifest, apply-mapping, addon-lock, addon-lock sidecar, and backup
  metadata validation to the read/write boundaries that own those contracts
- GitHub release fetch now validates non-empty release tags, portable case-insensitively unique
  asset names, and HTTP(S) asset download URLs; selecting a GitHub asset also rejects explicit
  non-zip assets before cache or downloader code sees them
- CurseForge file fetch now validates non-zero file ids, portable filenames, RFC 3339-shaped file
  dates, and HTTP(S) download URLs before selection; final selected files still enforce available
  `.zip` archive semantics
- the CurseForge provider now accepts `CURSEFORGE_API_KEY` as a fallback to the namespaced
  HearthSync API key env var, matching common local tooling conventions without giving up the
  application-specific variable
- shared boundary validators now own HTTP(S) URL scheme recognition and RFC 3339-shaped timestamp
  recognition, so backup metadata, addon source parsing, provider materialization, cache recovery,
  and remote provider DTO validation no longer carry parallel low-level parsers
- CurseForge search responses now validate non-zero mod ids, non-empty unpadded names, HTTP(S)
  website URLs, and non-zero latest-file index ids before projection into app-owned search results
- CurseForge game and game-version-type context responses now validate non-zero unique ids plus
  non-empty unpadded names/slugs before those ids are used in follow-up provider requests
- CurseForge search latest-file indexes validate non-zero ids without treating
  `gameVersionTypeId` as unique, matching observed API responses
- optional CurseForge search summaries are normalized during app-result projection: trimmed text is
  preserved, blank text becomes absent
- provider archive validators now fail closed before cache metadata persistence: GitHub asset
  size/digest/updated_at values and CurseForge file length/hash/sortable-version metadata are
  validated before initial materialization or cache repair can store them
- SHA-256 hex validation now lives in the shared boundary validator module and is reused by addon
  locks and bundle source indexes instead of duplicated locally
- CurseForge `releaseType` and dependency DTO fields now validate before release-channel filtering
  or dependency planning uses them; unknown positive dependency relation types remain allowed and
  ignored until HearthSync deliberately supports more than required dependencies
- CurseForge release, hash, and dependency relation policy semantics now live in
  `curseforge::policy`; the generic source adapter only projects filtered dependency mod ids into
  `AddonSourceRef` values
- provider-specific cache validator projection now lives with each provider: GitHub asset projection
  is in `github.rs`, CurseForge file projection is in `curseforge::policy`, and
  `provider::validation` only owns provider-agnostic cache/transport helpers
- the next provider-side candidate is checking whether provider-owned tests should move closer to
  the provider modules now that the generic provider test file is carrying many narrow boundary
  cases
