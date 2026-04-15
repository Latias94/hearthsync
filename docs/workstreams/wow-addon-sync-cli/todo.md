# WoW Addon Sync CLI TODO

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
- [ ] Classify WTF scopes as account-root files, account SavedVariables, role SavedVariables, role state, and cache-like files
- [ ] Add NewBeeBox-style `share` and `sync` semantics as explicit policy choices or aliases
- [x] Add delete operations to plans for mirror and replace policies
- [x] Introduce internal `BundleReader`, `BundlePlanner`, and `BundleExecutor` orchestration
- [ ] Split bundle archive reading, planning, staging, rewriting, and execution boundaries
- [x] Move rewrite materialization out of bundle planning
- [ ] Ensure plan generation never mutates target state
- [ ] Make bundle apply use one operation-level backup and rollback boundary
- [ ] Make addon lock apply use one operation-level backup and rollback boundary

## Phase 6 - Lua Rewrite Engine

- [x] Implement profile key detection and replacement
- [x] Implement character and server identity replacement
- [ ] Add file-level rewrite opt-in rules
- [ ] Add encoding-aware file read and write support
- [ ] Add byte-safe replacement support for WTF files that are not valid UTF-8
- [ ] Add regression tests with real-world samples
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

## Phase 8 - Reliability and Polish

- [ ] Add integration tests with fixture installs
- [x] Add rollback validation tests
- [ ] Add archive compatibility tests
- [ ] Add Windows to macOS migration scenario tests
- [ ] Replace whole-file zip writes with streaming archive I/O for large WTF files
- [ ] Introduce a core task model with progress events and cancellation tokens
- [ ] Keep provider networking behind traits before considering full async runtime adoption
- [ ] Add human-readable summary reports
- [ ] Add documentation for future `egui` integration

## Nice-to-Have

- [ ] Support remote bundle index files
- [ ] Support signed bundle manifests
- [ ] Support bundle diffing
- [ ] Support selective addon enable-state export and import
- [ ] Support richer addon-specific migration plugins
