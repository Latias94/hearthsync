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
- [x] Implement `hearthsync config preview`

## Phase 6 - Lua Rewrite Engine

- [x] Implement profile key detection and replacement
- [x] Implement character and server identity replacement
- [ ] Add file-level rewrite opt-in rules
- [ ] Add encoding-aware file read and write support
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
- [ ] Add human-readable summary reports
- [ ] Add documentation for future `egui` integration

## Nice-to-Have

- [ ] Support remote bundle index files
- [ ] Support signed bundle manifests
- [ ] Support bundle diffing
- [ ] Support selective addon enable-state export and import
- [ ] Support richer addon-specific migration plugins
