# WoW Addon Sync CLI Milestones

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

- staged extraction is implemented before plan generation and apply
- automatic backup before mutation is implemented
- automatic rollback on apply failure is implemented
- `--dry-run` preview is implemented
- explicit target remapping is implemented through CLI flags and a mapping file
- account-level target selection is now a confirmed product requirement from reference research
- `bundle plan` and `ApplyPlan` are implemented

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
- Lua rewrite currently covers profile-key style identities and quoted character/server strings
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

## M6 - Stabilization for GUI Reuse

### Goal

Prepare the core for a future `egui` frontend.

### Deliverables

- stable core APIs
- machine-readable progress events
- consistent error surfaces
- improved test coverage

### Exit Criteria

- core logic is reusable without CLI assumptions
- progress and result models are suitable for an `egui` frontend
