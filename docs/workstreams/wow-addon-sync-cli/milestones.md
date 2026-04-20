# WoW Addon Sync CLI Milestones

Reusable architecture ownership now lives in `../wow-addon-sync-core/`.
These milestones should describe CLI delivery on top of that shared core.

## M0 - Documentation Baseline

### Goal

Create a clear written architecture before implementation starts.

### Deliverables

- `design.md`
- `todo.md`
- `milestones.md`

### Exit Criteria

- scope is documented
- bundle model is documented
- first implementation phase is agreed

## M1 - Safe Core Skeleton

### Status

Completed on 2026-04-15

### Goal

Create the minimum safe CLI and engine skeleton.

### Deliverables

- CLI subcommand scaffold
- installation scanning
- installation inspection
- backup checkpoint creation
- manifest data structures

### Exit Criteria

- `hearthsync scan` works on at least one local installation
- `hearthsync inspect` prints structured installation details
- backup creation works before any mutating operation

### Current Notes

- `backup create`, `backup list`, and `backup restore` are implemented
- backup catalogs are read from zip metadata and sorted by creation time
- restore accepts either an explicit archive path or a `backup_id` resolved from the backup directory

## M2 - Portable Bundle Packaging

### Status

In progress

### Goal

Export a portable WoW setup bundle from a local installation.

### Deliverables

- `bundle pack`
- `bundle inspect`
- bundle manifest validation
- include and exclude filters

### Exit Criteria

- a bundle can be created from a Windows installation
- the produced archive contains a valid manifest and normalized layout
- package contents can be inspected without applying them

### Current Notes

- `bundle pack` is implemented
- `bundle inspect` is implemented
- normalized archive layout is covered by automated tests
- bundle archives can embed addon lock and addon index sidecar metadata
- include and exclude glob filters are still pending

## M3 - Portable Bundle Apply

### Status

In progress

### Goal

Apply a portable bundle to another installation safely.

### Deliverables

- staged extraction
- apply plan
- dry-run preview
- ordered resource application
- rollback support

### Exit Criteria

- a bundle exported on Windows can be applied on macOS in a controlled way
- the tool creates a backup checkpoint automatically
- users can preview changes before mutation

### Current Notes

- bundle preview planning now reads archive metadata and entry bytes without staged extraction
- execution materializes archive entries and Lua rewrites only when mutation is actually performed
- automatic backup before mutation is implemented
- automatic rollback on apply failure is implemented
- `--dry-run` preview is implemented
- explicit target remapping is implemented through CLI flags and a mapping file
- account-level target selection is now a confirmed product requirement from reference research
- `bundle plan` and `ApplyPlan` are implemented
- bundle unpack preserves embedded addon metadata under `.hearthsync/bundles/<bundle-id>/` without overwriting active addon tracking
- bundle addon plan/apply can read embedded addon locks directly from the archive and reuse the lock sync engine
- bundle-local addon source archives allow cross-machine addon lock sync without requiring source-machine local paths
- direct `external-package plan/apply` now defaults to the shared author-package profile from the core workstream instead of merge-first behavior
- `external-package plan/apply` now supports manifest metadata overrides and per-resource apply-policy overrides for normalized imports
- Windows-source to macOS-target migration coverage now includes external-package policy override scenarios in automated tests

## M3.5 - Sync Semantics Hardening

### Status

Active - blocking

### Goal

Make bundle and addon synchronization behavior explicit, previewable, and transaction-oriented.

### Deliverables

- resource-group apply policy model
- manifest apply intent consumed by runtime behavior
- logical plan model with add, replace, remove, skip, and rewrite operations
- separated bundle reader, planner, and executor boundaries
- operation-level backup and rollback for bundle apply
- operation-level backup and rollback for addon lock apply
- WTF file classification for account-root state, account SavedVariables, role SavedVariables, role state, and cache-like files
- explicit `share` and `sync` semantics informed by NewBeeBox research

### Exit Criteria

- users can preview stale files that will be deleted before apply
- legacy `replace_addons`, `replace_fonts`, and `merge_wtf` flags are removed in favor of a policy enum
- bundle plan generation does not mutate target state
- a failed bundle or addon lock apply rolls back the whole operation
- core request/result types are stable enough to be consumed by a future frontend
- `WTF/Account/SavedVariables` is not misclassified as a playable account

### Current Notes

- This milestone is now the active blocker for the rest of the `wow-addon-sync-cli` workstream.
- New CLI surface area should only be added when it directly supports sync semantics hardening.
- future frontend work must wait until planner/executor boundaries and operation-level rollback semantics are stable.
- The expected refactor style is in-place simplification, including removal of obsolete prototype paths.
- Bundle preview is now execution-independent and no longer depends on staged files in the public plan.
- Bundle archive apply and direct external-package apply now share one internal source boundary for logical entry enumeration, preview byte reads, and execution materialization.
- Bundle apply and addon lock apply both use a single backup and rollback boundary.
- `keep_original`, `explicit`, and `prompt` now affect runtime mapping behavior instead of being partially ignored.
- `prompt` currently means the caller must resolve mappings before plan/apply; the CLI does not open interactive prompts yet.
- Lua rewrite now only targets explicit account/character `SavedVariables` Lua files instead of every `.lua` payload in the bundle.
- Lua rewrite now has a byte-safe fallback for invalid UTF-8 and Latin-1-compatible payloads, while broader encoding support is still pending.
- A local scan of 400 retail `SavedVariables` Lua files found no UTF-16 BOM samples, no high-NUL text payloads, and one invalid UTF-8 file (`Auctionator.lua`).
- Lua rewrite now requires either explicit content markers such as `profileKeys` / `realm` or a small known-file rule set such as `MeetingStone.lua`.
- A targeted local scan also found `EventsTracker.lua` and `SavedInstances.lua` storing quoted character-realm keys without those generic markers, so they stay on the explicit rule list.
- this does not justify a new workstream; the next bounded slice stays in the existing core/CLI sync-hardening track
- the next bounded slice starts with shared addon-root detection for real-world archives whose `.toc` file name differs from the directory name, then continues into root-level `WTF/Account/SavedVariables` import support
- the next blocking slice after that is portable path semantics: external-package path sets that only
  differ by case must not survive into apply on Windows/default macOS targets, and addon-index local
  archives must not depend on the caller running from the index directory

## M4 - Configuration Sync Engine

### Status

In progress

### Goal

Handle account and character configuration migration reliably.

### Deliverables

- common WTF sync
- character WTF sync
- targeted Lua rewrite engine
- character mapping workflow

### Exit Criteria

- character-targeted import works with explicit mapping
- `AddOns.txt` state is preserved when requested
- profile key rewrites succeed on representative samples

### Current Notes

- character-targeted import works for the current CLI through account/server/character remapping
- Lua rewrite currently covers profile-key style identities and quoted character/server strings within explicit `SavedVariables` allowlist paths
- Lua rewrite now defaults unknown `SavedVariables` files to copy-only unless a rule signal matches
- Lua rewrite now preserves non-text bytes around matched replacements and can rewrite some non-UTF-8 payloads without decoding the whole file
- Known-file rewrite exceptions are now centralized in one explicit rule table instead of being scattered across ad hoc checks
- Anonymized fixtures derived from local `MeetingStone.lua`, `EventsTracker.lua`, `SavedInstances.lua`, and invalid-UTF-8 `Auctionator.lua` samples are now checked into `lua_patch` tests
- common `WTF` overwrite is account-selective instead of global-only
- addon-specific rewrite plugins and richer encoding handling are still pending

## M5 - Addon Management

### Status

In progress

### Goal

Add direct addon install and update capabilities.

### Deliverables

- addon source abstraction
- addon install command
- addon update command
- addon removal command

### Exit Criteria

- users can manage individual addons without using bundle workflows
- install and update reporting is clear and scriptable

### Current Notes

- `addon search`, `addon list`, `addon install`, `addon update`, and `addon remove` are implemented
- the first source abstraction supports local zip archives, direct `http/https` zip downloads, `GitHub Releases`, and `CurseForge` shortcut sources
- custom addon indexes support curated inspect/install/update workflows without requiring provider search access
- CurseForge source resolution now filters files by the target WoW flavor through official version-type metadata
- addon receipts are tracked in `Interface/AddOns/.hearthsync/addons.toml`
- a derived addon lock is tracked in `Interface/AddOns/.hearthsync/lock.toml` with installed content hashes and curated index metadata
- addon lock diff/verify can detect cross-machine differences, local content drift, missing tracked directories, and untracked addon directories
- addon lock plan/apply can turn a lock file into concrete install/update/remove actions for another machine
- addon updates reuse the recorded source reference and refresh tracked addon directories
- addon removal also cleans the receipt registry when the last tracked package is removed
- richer provider-specific metadata is still pending

## M6 - Stabilization for Frontend Reuse

### Goal

Prepare the core for a future frontend.

### Deliverables

- stable core APIs
- machine-readable progress events
- cancellation token model
- task abstraction for long-running bundle/addon operations
- provider/network abstraction that can later support async implementations
- consistent error surfaces
- improved test coverage

### Exit Criteria

- core logic is reusable without CLI assumptions
- progress and result models are suitable for a future frontend
- CLI can run tasks synchronously while a future frontend can run the same task model on worker threads
- M3.5 sync semantics hardening is complete before frontend implementation begins

### Current Notes

- stable CLI handlers that need shared runtime policy now construct `core::app::StableAppServices`,
  while addon-index/addon-lock/bundle-addon-lock commands still use `core::app::ExtendedAppServices`
  for the less-stable app surface
- CLI command handlers now share one `cli::app_support` helper for service construction and
  installation resolution, so command modules no longer duplicate `ResolveInstallationRequest`
  assembly or drift on which app boundary they should enter through
- addon-lock CLI text rendering now also shares formatter helpers under `cli::output`, so diff,
  verify, apply, and plan views no longer keep near-duplicate package/change rendering logic or
  header-summary assembly in individual command handlers, including bundle-addon lock flows
- addon-index CLI text rendering now also shares formatter helpers under `cli::output`, so
  inspect/install/update handlers keep only request assembly, app calls, and the final shared
  `render(...)` dispatch
- addon search/list/install/update/remove CLI text rendering now also shares formatter helpers
  under `cli::output`, so addon management handlers no longer keep one-off inventory, package, or
  backup-summary string assembly inline
- stable system/backup CLI text rendering now also shares formatter helpers under `cli::output`,
  so install scan/inspect/doctor and backup create/list/restore stop carrying one-off text
  formatting logic in their command handlers
- bundle archive/apply/external-package CLI text rendering now also shares formatter helpers under
  `cli::output`, and character-mapping plus external-package warning text formatting is owned there
  too, so those command handlers no longer duplicate account, mapping, or warning summary assembly
- the current CLI command handlers no longer keep inline `render(json, ..., |item| ...)` text
  closures; human-readable rendering is now concentrated under `cli::output`
- `cli::output` is now split by domain modules (`addon`, `addon_lock`, `bundle`,
  `external_package`, `system`, `backup`, `shared`), with `output.rs` reduced to a stable API
  shell and formatter tests colocated with their domain renderers, so future CLI text changes can
  stay localized instead of reopening one growing monolithic file
- `cli::args` is now also split by domain modules (`addon`, `backup`, `bundle`,
  `external_package`, `shared`), with `args.rs` reduced to top-level command routing plus
  re-exports, so Clap surface growth no longer forces one monolithic argument-definition file
- external-package CLI request projection is now isolated in `cli::external_package::request`,
  while external-package warning formatter tests live with `cli::output::shared`, keeping command
  handling, app-request projection, and formatter coverage under their owning modules
- bundle-apply and addon-manage CLI request projection is now also isolated in
  `cli::bundle_apply::request` and `cli::addon_manage::request`, so those handlers keep only
  installation resolution, request assembly dispatch, app calls, and final rendering
- apply-mapping file loading plus CLI override merging now lives in shared `cli::mapping`, so
  bundle-apply and external-package no longer couple to helper ownership in a sibling command
  module
- addon-lock, addon-index, backup, bundle-addon, bundle-archive, system, and the remaining
  external-package request projection now also live in domain `request` modules, so inline
  app-request struct assembly is effectively gone from `src/cli` except for the shared
  installation-resolution helper in `cli::app_support`
- repeated CLI install-target and apply-mapping flags now live in shared `cli::args::shared`
  shapes (`InstallTargetArgs`, `ApplyMappingArgs`), and the shared installation-resolution plus
  mapping helpers now consume those shapes directly, reducing argument-definition drift between
  addon, bundle, backup, system, and external-package commands
- `cli::output` root now re-exports domain renderers directly, so the module remains a thin API
  surface around shared JSON/text dispatch instead of duplicating one-line forwarding functions for
  every formatter
- bundle-apply and external-package CLI handlers now also share one apply-target resolution helper
  in `cli::app_support`, so installation resolution plus apply-mapping resolution no longer drift
  between those two plan/apply entrypoints
- install-target based CLI handlers now also share execution helpers in `cli::app_support`
  (`render_with_installation`, `render_with_apply_target`), reducing repeated resolve-target →
  app-call → render control flow across addon, addon-index, addon-lock, backup, and bundle flows
- manifest example/validate now also project through `cli::system::request` and
  `cli::output::system`, so the remaining system-only manifest commands follow the same shared
  result/render boundary as the rest of the CLI
- addon and bundle top-level routers now dispatch directly into variant-specific handlers, removing
  the old subordinate “internal CLI routing error” dead branches and making command ownership more
  explicit at the routing boundary
- fallible CLI request projection now uses an explicit `render_with_fallible_installation(...)`
  helper, so `bundle pack` keeps manifest loading errors in the request-building step instead of
  passing `AppResult` as a pseudo-request into the app invocation closure
- external package source discovery and safe source-path normalization now live under
  `core::bundle::external_package::source`, starting the split of the large author-package import
  module into focused pipeline stages without changing runtime behavior
- external package classification and warning production now live under
  `core::bundle::external_package::classify`, keeping addon/WTF/fonts/interface recognition
  separate from source enumeration and later bundle materialization
- external package staging-installation creation and normalized file materialization now live under
  `core::bundle::external_package::materialize`, keeping import execution mechanics separate from
  analysis, manifest construction, and app-facing projection
- external package derived analysis data and generated manifest construction now live under
  `core::bundle::external_package::analysis` and `core::bundle::external_package::manifest`, so
  summary/resource projection and author-package defaults are isolated from command orchestration
- external package normalized-path validation/source mapping and app-facing plan/apply result
  projection now live under `core::bundle::external_package::normalized` and
  `core::bundle::external_package::projection`, further reducing the main module to pipeline
  orchestration
- external package public request/result DTOs now live under
  `core::bundle::external_package::types`, separating stable API shape from import pipeline
  implementation details
- external package progress/cancellation wrappers now live under
  `core::bundle::external_package::tasks`, keeping task reporting separate from pure
  analyze/create/plan orchestration
- bundle planner internal state shapes now live under `core::bundle::planner::model`, preparing
  the next split between logical planning and preview finalization
- bundle planner preview finalization now lives under `core::bundle::planner::preview`, so logical
  entry planning no longer owns byte-comparison and final apply-plan projection details
- bundle planner logical apply construction now lives under `core::bundle::planner::logical`,
  leaving the planner root focused on bundle/external source entry orchestration
- bundle planner source/entry-reader orchestration now lives under
  `core::bundle::planner::pipeline`, so the planner root is now just a re-export shell over the
  split planning phases
- bundle apply task message/context policy now lives under `core::bundle::apply::task_context`,
  separating shared bundle/external-package progress wording from the filesystem execution flow
- bundle apply filesystem execution, backup creation, and rollback handling now live under
  `core::bundle::apply::executor`, leaving the apply root focused on task entrypoints and result
  projection
- bundle apply dry-run/execute result projection now lives under
  `core::bundle::apply::result`, so the root apply flow no longer assembles `UnpackedBundle`
  inline
- shared prepared-apply orchestration for bundle archives and external packages now lives under
  `core::bundle::apply::pipeline`, leaving `apply.rs` as the bundle-specific task entry shell
- apply-source bundle-archive and external-package read/materialize paths now live under
  `core::bundle::apply_source::bundle_archive` and
  `core::bundle::apply_source::external_package`, so `apply_source.rs` is reduced to source-kind
  dispatch
- external package manifest preparation, normalized-path validation, and prepared-apply assembly
  now live under `core::bundle::external_package::prepare`, reducing `external_package.rs` to the
  app-facing analyze/create/plan shell around the split import pipeline
- bundle packing output-path/timestamp policy and archive inspection helpers now live under
  `core::bundle::packing::output` and `core::bundle::packing::inspect`, so `packing.rs` stays
  focused on archive creation instead of also owning naming and read-only inspection concerns
- bundle entry planning now separates archive-entry dispatch, common account replication, and
  character mapping under `core::bundle::entry_plan::{context,common,character}`, leaving
  `entry_plan.rs` as the small planning-context shell
- bundle archive reading now separates entry-byte/materialization helpers, manifest plus entry-count
  inspection, and embedded addon-lock extraction under
  `core::bundle::archive_read::{entries,inspect,addon_lock}`, leaving `archive_read.rs` as the
  small re-export shell
- bundle character mapping now separates target-resolution mechanics and input/error validation
  under `core::bundle::character_mapping::{resolution,validation}`, leaving
  `character_mapping.rs` focused on per-resource mode dispatch
- bundle target-account logic now separates common-account selection, source-account target
  inference, and installation compatibility checks under
  `core::bundle::target_accounts::{selection,common,compatibility}`, leaving
  `target_accounts.rs` as the thin re-export shell
- bundle apply policy logic now separates cleanup-root planning, resource-group policy lookup, and
  operation/group ordering under `core::bundle::apply_policy::{cleanup,policy,order}`, leaving
  `apply_policy.rs` as the thin re-export shell
- bundle addon-source archive support now separates addon-index path resolution, generated
  addon-lock loading, and embedded source-archive materialization under
  `core::bundle::addon_source_archive::{index_paths,lock,source_bundle}`, leaving
  `addon_source_archive.rs` as the thin re-export shell
- bundle execution now separates filesystem apply/materialization, target-byte comparison, and
  rollback error wrapping under `core::bundle::execution::{apply,compare,rollback}`, leaving
  `execution.rs` as the thin re-export shell
- bundle WTF archive support now separates common WTF packing, character WTF packing, and
  source-account resolution under `core::bundle::wtf_archive::{common,character,resolve}`,
  leaving `wtf_archive.rs` as the thin re-export shell
- bundle public DTOs now separate archive pack/inspect models, bundle-addon-lock models, and
  bundle-apply models under `core::bundle::types::{archive,addon_lock,apply}`, leaving
  `types.rs` as the thin re-export shell
- bundle shared helpers now separate addon-source index models, path-safety utilities, and zip
  option helpers under `core::bundle::shared::{addon_source_index,path,zip_options}`, leaving
  `shared.rs` as the thin re-export shell
- bundle apply preparation models now separate planned-entry state, preview operations, and
  prepared execution state under `core::bundle::apply_model::{planned,preview,prepared}`,
  leaving `apply_model.rs` as the thin re-export shell
- bundle packing now separates archive write orchestration and resource-specific zip
  materialization under `core::bundle::packing::{pack,resources}`, leaving `packing.rs` as the
  thin re-export shell beside output and inspect helpers
- bundle apply-source dispatch now separates cross-source reader state and prepared-source method
  dispatch under `core::bundle::apply_source::{reader,dispatch}`, leaving `apply_source.rs` as
  the thin shell beside bundle-archive and external-package source adapters
- bundle root module now separates archive constants, public API exports, and internal legacy
  prelude imports under `core::bundle::{constants,exports,imports}`, leaving `mod.rs` focused on
  module wiring while existing `super::*` call sites are progressively retired
- install discovery is now app-first from the CLI perspective; the reusable frontend-facing route is
  the direct installation surface on `core::app::StableAppServices`, not old direct domain helpers
- frontend stabilization is now mainly gated by planner-boundary cleanup and stronger app-contract
  ownership, not by the already-addressed portable path and provider-cancellation basics
