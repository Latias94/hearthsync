# WoW Addon Sync CLI TODO

## Current Focus

The active priority is sync semantics hardening inside the existing CLI workstream.
Before adding more CLI surface area or starting any future frontend work, the core must stabilize planning, execution, rollback, and manifest runtime semantics.
Reusable architecture ownership now lives in `../wow-addon-sync-core/`.
This file should focus on CLI-facing consequences of that work.

The current blocking sequence is:

1. remove remaining dead or duplicated transition paths after planner/executor stabilization
2. harden real-world addon package compatibility and remove duplicated classifier paths
3. harden portable author-package and addon-index path semantics across Windows/macOS targets
4. tighten Lua rewrite scope and encoding handling

## Phase 0 - Documentation and Alignment

- [x] Define the workstream scope
- [x] Record the first architecture draft
- [x] Record milestones and sequencing
- [ ] Confirm preferred bundle naming and UX wording

## Phase 1 - Project Skeleton

- [x] Add a CLI framework and subcommand layout
- [x] Create core module boundaries for install, bundle, backup, and sync
- [x] Add structured error types
- [ ] Add logging and verbose output modes
- [x] Add JSON output mode for machine-readable automation

## Phase 2 - Installation Discovery

- [x] Detect WoW installations on Windows
- [x] Detect WoW installations on macOS
- [x] Detect flavor roots such as retail and classic
- [x] Validate required subfolders for each installation
- [x] Implement `hearthsync scan`
- [x] Implement `hearthsync inspect --install <path>`
- [x] Implement `hearthsync doctor --install <path>`

## Phase 3 - Bundle Manifest and Safety

- [x] Define `manifest.toml` types
- [x] Add manifest validation
- [x] Add staging directory lifecycle management
- [x] Add backup checkpoint creation
- [x] Add backup listing and restore support
- [x] Add dry-run planning model

## Phase 4 - Bundle Packing

- [x] Implement addon collection from `Interface/AddOns`
- [x] Implement common WTF collection
- [x] Implement character WTF collection
- [x] Implement fonts collection
- [x] Implement interface asset collection
- [ ] Add include and exclude glob support
- [x] Exclude obvious cache and transient files by default
- [x] Implement `hearthsync bundle pack`
- [x] Implement `hearthsync bundle inspect`
- [x] Embed addon lock and addon index sidecar metadata in bundles
- [x] Embed bundle-local addon source archives for lock-driven sync

## Phase 5 - Bundle Apply

- [x] Implement bundle extraction to staging
- [x] Implement compatibility checks by flavor
- [x] Implement target character selection inputs
- [x] Implement target account discovery and selection
- [x] Build an ordered `ApplyPlan`
- [x] Apply addon resources
- [x] Apply fonts and interface assets
- [x] Apply common WTF resources
- [x] Apply character WTF resources
- [x] Implement `hearthsync bundle unpack`
- [x] Unpack addon metadata sidecars without overwriting active addon tracking
- [x] Implement `hearthsync bundle addon-plan` and `bundle addon-apply` shortcuts
- [ ] Decide whether standalone `hearthsync config preview` is still needed or whether `bundle plan` owns preview UX

## Phase 5.5 - Sync Semantics Hardening

- [x] Fix account discovery so `WTF/Account/SavedVariables` is not treated as a playable account
- [x] Define explicit resource-group policies: `merge`, `mirror`, `replace_selected`, and `preserve`
- [x] Consume or replace manifest apply flags such as `replace_addons`, `replace_fonts`, and `merge_wtf`
- [x] Classify WTF scopes as account-root files, account SavedVariables, role SavedVariables, role state, and cache-like files
- [x] Add NewBeeBox-style `share` and `sync` semantics as explicit policy choices or aliases
- [x] Add delete operations to plans for mirror and replace policies
- [x] Introduce internal `BundleReader`, `BundlePlanner`, and `BundleExecutor` orchestration
- [x] Split bundle archive reading, planning, staging, rewriting, and execution boundaries
- [x] Introduce a pure preview plan that does not depend on staged execution files
- [x] Move rewrite materialization out of bundle planning
- [x] Ensure plan generation never mutates target state
- [x] Make bundle apply use one operation-level backup and rollback boundary
- [x] Make addon lock apply use one operation-level backup and rollback boundary
- [ ] Remove dead or duplicated transition paths after planner/executor boundaries are stable
  - Current cleanup: bundle archives and direct external packages now share one internal apply-source boundary for entry enumeration, preview byte reads, and execution materialization.
  - Current cleanup: the remaining low-level planner byte-reader seam is now test-only; stable CLI/frontend callers should stay on `core::app::StableAppServices`, and only reach for `core::app::ExtendedAppServices` plus its explicit `stable()` bridge when they need addon-index/addon-lock/bundle-addon-lock behavior.
  - Current cleanup: CLI service construction and installation-target resolution now share `cli::app_support`, so stable and extension commands no longer hand-roll slightly different app-entry glue.
  - Current cleanup: stable `system` and `backup` commands now also use shared formatter helpers in `cli::output`, so base installation/backup flows no longer own private text-rendering functions.
  - Current cleanup: addon-lock and bundle-addon-lock CLI output now share formatter helpers in `cli::output`, reducing duplicate diff/verify/apply package rendering plus header-summary assembly and making future text-output changes cheaper to keep consistent.
  - Current cleanup: addon-index CLI output now also shares formatter helpers in `cli::output`, so inspect/install/update handlers no longer carry inline string assembly closures.
  - Current cleanup: addon search/list/install/update/remove CLI output now also shares formatter helpers in `cli::output`, so addon inventory and package-operation summaries are no longer built inline in the handler.
  - Current cleanup: bundle archive/apply/external-package CLI output now also shares formatter helpers in `cli::output`, and the shared character-mapping plus external-package warning formatters now live there so those command modules stop depending on helper ownership scattered across sibling files.
  - Current cleanup: there are currently no remaining inline `render(json, ..., |item| ...)` text closures under `src/cli`; `cli::output` is now split by domain (`addon`, `addon_lock`, `bundle`, `external_package`, `system`, `backup`, `shared`), while `output.rs` stays as the thin API shell and formatter tests live beside their domain modules with shared test fixtures.
  - Current cleanup: `cli::args` is now also split by domain (`addon`, `backup`, `bundle`, `external_package`, `shared`), so the root args file only owns top-level routing plus re-exports instead of carrying every Clap enum and conversion in one place.
  - Current cleanup: external-package CLI request projection now lives in its own `cli::external_package::request` module, and external-package warning formatter tests moved back under `cli::output::shared`.
  - Current cleanup: bundle-apply and addon-manage CLI request projection now live in `cli::bundle_apply::request` and `cli::addon_manage::request`, so those handlers keep only install resolution, request assembly dispatch, app calls, and final rendering.
  - Current cleanup: apply-mapping file loading plus CLI override merging now belongs to shared `cli::mapping`, so bundle-apply and external-package flows no longer depend on helper ownership in a sibling command module.
  - Current cleanup: addon-lock, addon-index, backup, bundle-addon, bundle-archive, system, and the remaining external-package request projection now also live in domain `request` modules, so inline app-request struct assembly is effectively gone from `src/cli` except for the shared installation-resolution helper in `cli::app_support`.
  - Current cleanup: repeated CLI `--install/--flavor` and bundle/external-package apply-mapping flags now live in shared `cli::args::shared` structs (`InstallTargetArgs`, `ApplyMappingArgs`), and `cli::app_support` plus `cli::mapping` now consume those shared shapes directly, so handler signatures and Clap definitions stop drifting across commands.
  - Current cleanup: `cli::output` root now re-exports domain renderers directly instead of wrapping each one-line forwarding function, so the module stays a thin API surface around `render(json, ...)` rather than another growing list of pass-through bodies.
  - Current cleanup: bundle-apply and external-package CLI handlers now also share one `resolve_cli_apply_target(...)` path in `cli::app_support`, so installation resolution and apply-mapping resolution no longer drift between those two plan/apply entrypoints.
  - Current cleanup: install-target based CLI handlers now also share `render_with_installation(...)` and apply-target handlers share `render_with_apply_target(...)` in `cli::app_support`, so addon/addon-index/addon-lock/backup/bundle command modules no longer repeat the same resolve-target → invoke app → render control flow.
  - Current cleanup: manifest example/validate now also flow through `cli::system::request` plus `cli::output::system`, so even the remaining system-only manifest commands no longer hand-roll JSON/text printing outside the shared renderer boundary.
  - Current cleanup: addon and bundle top-level routers now dispatch directly into variant-specific handlers, deleting the old “internal CLI routing error” dead branches from subordinate command modules.
  - Current cleanup: fallible CLI request projection now has an explicit `render_with_fallible_installation(...)` helper, so `bundle pack` no longer smuggles `AppResult` through the normal request generic before invoking the app service.
  - Current cleanup: external package source discovery and safe path normalization now live in `core::bundle::external_package::source`, beginning the split of the large author-package import module into smaller pipeline stages.
  - Current cleanup: external package classification and warning production now live in `core::bundle::external_package::classify`, separating source enumeration from addon/WTF/fonts/interface resource recognition.
  - Current cleanup: external package staging-installation creation and source-to-target file materialization now live in `core::bundle::external_package::materialize`, keeping normalized import execution separate from analysis and manifest construction.
  - Current cleanup: external package analysis summarization/resource projection and generated manifest construction now live in `core::bundle::external_package::analysis` and `core::bundle::external_package::manifest`, so the main module keeps less derived-data assembly inline.
  - Current cleanup: external package normalized-path validation/source mapping and app-facing result projection now live in `core::bundle::external_package::normalized` and `core::bundle::external_package::projection`, leaving the main module closer to pure request orchestration.
  - Current cleanup: external package public request/result DTOs now live in `core::bundle::external_package::types`, so API shape is separated from the import pipeline implementation.
  - Current cleanup: external package progress/cancellation wrappers now live in `core::bundle::external_package::tasks`, separating task reporting from pure analyze/create/plan orchestration.
  - Current cleanup: bundle planner internal state shapes now live in `core::bundle::planner::model`, preparing the next split between logical planning and preview finalization.
  - Current cleanup: bundle planner preview finalization now lives in `core::bundle::planner::preview`, so logical entry planning no longer owns byte-comparison and final apply-plan projection details.
  - Current cleanup: bundle planner logical apply construction now lives in `core::bundle::planner::logical`, leaving the planner root focused on bundle/external source entry orchestration.
  - Current cleanup: bundle planner source/entry-reader orchestration now lives in `core::bundle::planner::pipeline`, so the planner root is now just a re-export shell over the split planning phases.
  - Current cleanup: bundle apply task message/context policy now lives in `core::bundle::apply::task_context`, separating shared bundle/external-package progress wording from the filesystem execution flow.
  - Current cleanup: bundle apply filesystem execution, backup creation, and rollback handling now live in `core::bundle::apply::executor`, leaving the apply root focused on task entrypoints and result projection.
  - Current cleanup: bundle apply dry-run/execute result projection now lives in `core::bundle::apply::result`, so the root apply flow no longer assembles `UnpackedBundle` inline.
  - Current cleanup: shared prepared-apply orchestration for bundle archives and external packages now lives in `core::bundle::apply::pipeline`, leaving `apply.rs` as the bundle-specific task entry shell.
  - Current cleanup: apply-source bundle-archive and external-package read/materialize paths now live in `core::bundle::apply_source::bundle_archive` and `core::bundle::apply_source::external_package`, so `apply_source.rs` is reduced to dispatch over source kind.
  - Current cleanup: external package manifest/normalized-path validation and prepared-apply assembly now live in `core::bundle::external_package::prepare`, reducing `external_package.rs` to app-facing analyze/create/plan entry orchestration.
  - Current cleanup: bundle packing output-path/timestamp policy and archive inspection helpers now live in `core::bundle::packing::output` and `core::bundle::packing::inspect`, leaving `packing.rs` focused on archive creation.
  - Current cleanup: bundle entry planning now separates archive-entry dispatch, common account replication, and character mapping into `core::bundle::entry_plan::{context,common,character}`, leaving `entry_plan.rs` as a small planning-context shell.
  - Current cleanup: bundle archive reading now separates entry-byte/materialization helpers, manifest/entry-count inspection, and embedded addon-lock extraction into `core::bundle::archive_read::{entries,inspect,addon_lock}`, leaving `archive_read.rs` as a re-export shell.
  - Current cleanup: bundle character mapping now separates target-resolution mechanics and input/error validation into `core::bundle::character_mapping::{resolution,validation}`, leaving `character_mapping.rs` focused on per-resource mode dispatch.
  - Current cleanup: bundle target-account logic now separates common-account selection, source-account target inference, and installation compatibility checks into `core::bundle::target_accounts::{selection,common,compatibility}`, leaving `target_accounts.rs` as a thin re-export shell.
  - Current cleanup: bundle apply policy logic now separates cleanup-root planning, resource-group policy lookup, and operation/group ordering into `core::bundle::apply_policy::{cleanup,policy,order}`, leaving `apply_policy.rs` as a thin re-export shell.
  - Current cleanup: bundle addon-source archive support now separates addon-index path resolution, generated addon-lock loading, and embedded source-archive materialization into `core::bundle::addon_source_archive::{index_paths,lock,source_bundle}`, leaving `addon_source_archive.rs` as a thin re-export shell.
  - Current cleanup: bundle execution now separates filesystem apply/materialization, target-byte comparison, and rollback error wrapping into `core::bundle::execution::{apply,compare,rollback}`, leaving `execution.rs` as a thin re-export shell.
  - Current cleanup: bundle WTF archive support now separates common WTF packing, character WTF packing, and source-account resolution into `core::bundle::wtf_archive::{common,character,resolve}`, leaving `wtf_archive.rs` as a thin re-export shell.
  - Current cleanup: bundle public DTOs now separate archive pack/inspect models, bundle-addon-lock models, and bundle-apply models into `core::bundle::types::{archive,addon_lock,apply}`, leaving `types.rs` as a thin re-export shell.
  - Current cleanup: bundle shared helpers now separate addon-source index models, path-safety utilities, and zip option helpers into `core::bundle::shared::{addon_source_index,path,zip_options}`, leaving `shared.rs` as a thin re-export shell.
- [x] Replace duplicated addon-root detection with one shared classifier reused by addon install and external-package import
- [x] Support addon archives whose `.toc` file name differs from the directory name
- [x] Normalize `WTF/Account/SavedVariables` external-package imports instead of warning-only drop
- [x] Reject external-package path sets that would collide on case-insensitive Windows/default macOS targets
- [x] Make manifest character mapping intent (`keep_original`, `explicit`, `prompt`) affect runtime behavior
- [x] Align apply ordering with explicit resource-group ordering instead of archive iteration order

## Phase 6 - Lua Rewrite Engine

- [x] Implement profile key detection and replacement
- [x] Implement character and server identity replacement
- [x] Add file-level rewrite opt-in rules
- [ ] Add encoding-aware file read and write support
- [x] Add byte-safe replacement support for WTF files that are not valid UTF-8
- [x] Add regression tests with real-world samples
- [x] Add account-level overwrite rules for common WTF resources

## Phase 7 - Addon Management

- [x] Implement addon source abstraction
- [x] Implement addon install flow
- [x] Implement addon update flow
- [x] Implement addon removal flow
- [x] Implement addon search flow
- [x] Implement CurseForge flavor-aware file selection
- [x] Implement custom addon index inspect/install/update
- [x] Implement derived addon lock inspect/write workflow
- [x] Implement addon lock diff/verify workflow
- [x] Implement addon lock plan/apply workflow
- [x] Implement `hearthsync addon list`
- [x] Implement `hearthsync addon search`
- [x] Implement `hearthsync addon install`
- [x] Implement `hearthsync addon update`
- [x] Implement `hearthsync addon remove`
- [x] Resolve addon-index relative local archives against the index file instead of the caller working directory

## Phase 8 - Reliability and Polish

- [ ] Add integration tests with fixture installs
- [x] Add rollback validation tests
- [ ] Add archive compatibility tests
- [x] Add Windows to macOS migration scenario tests
- [ ] Replace whole-file zip writes with streaming archive I/O for large WTF files
- [x] Introduce a core task model with progress events and cancellation tokens
- [x] Keep provider networking behind traits before considering full async runtime adoption
- [x] Add direct `external-package inspect/plan/apply` command surface on top of the normalized import pipeline
- [x] Add metadata and apply-policy override flags for `external-package plan/apply`
- [x] Consume one shared core app entrypoint instead of constructing unrelated service facades per handler
- [x] Make provider-backed addon downloads timeout-bounded and truthfully cancellable before GUI reuse
- [ ] Add human-readable summary reports
- [ ] Add documentation for future frontend integration

## Nice-to-Have

- [ ] Support remote bundle index files
- [ ] Support signed bundle manifests
- [ ] Support bundle diffing
- [ ] Support selective addon enable-state export and import
- [ ] Support richer addon-specific migration plugins
