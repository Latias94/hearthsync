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
- current bundle apply semantics are still merge-copy oriented and must be hardened before GUI work

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
- core request/result types are stable enough to be consumed by a future `egui` frontend
- `WTF/Account/SavedVariables` is not misclassified as a playable account

### Current Notes

- This milestone is now the active blocker for the rest of the `wow-addon-sync-cli` workstream.
- New CLI surface area should only be added when it directly supports sync semantics hardening.
- `egui` work must wait until planner/executor boundaries and operation-level rollback semantics are stable.
- The expected refactor style is in-place simplification, including removal of obsolete prototype paths.
- Bundle preview is now execution-independent and no longer depends on staged files in the public plan.
- Bundle apply and addon lock apply both use a single backup and rollback boundary.
- `keep_original`, `explicit`, and `prompt` now affect runtime mapping behavior instead of being partially ignored.
- `prompt` currently means the caller must resolve mappings before plan/apply; the CLI does not open interactive prompts yet.
- Lua rewrite now only targets explicit account/character `SavedVariables` Lua files instead of every `.lua` payload in the bundle.
- Lua rewrite now has a byte-safe fallback for invalid UTF-8 and Latin-1-compatible payloads, while broader encoding support is still pending.
- A local scan of 400 retail `SavedVariables` Lua files found no UTF-16 BOM samples, no high-NUL text payloads, and one invalid UTF-8 file (`Auctionator.lua`).
- Lua rewrite now requires either explicit content markers such as `profileKeys` / `realm` or a small known-file rule set such as `MeetingStone.lua`.
- A targeted local scan also found `EventsTracker.lua` and `SavedInstances.lua` storing quoted character-realm keys without those generic markers, so they stay on the explicit rule list.

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
- cancellation token model
- task abstraction for long-running bundle/addon operations
- provider/network abstraction that can later support async implementations
- consistent error surfaces
- improved test coverage

### Exit Criteria

- core logic is reusable without CLI assumptions
- progress and result models are suitable for an `egui` frontend
- CLI can run tasks synchronously while GUI can run the same task model on worker threads
- M3.5 sync semantics hardening is complete before GUI implementation begins
