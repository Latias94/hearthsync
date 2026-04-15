# WoW Addon Sync CLI Design

## Status

Draft v0.1

## Problem Statement

`hearthsync` is intended to become a cross-platform World of Warcraft addon and configuration sync tool.
The first deliverable is a CLI. A future GUI will be built with `egui`, but the core logic must remain UI-agnostic.

The tool must support:

- downloading and updating addons
- exporting a local addon setup into a portable bundle
- importing a bundle on another machine, including Windows to macOS migration
- synchronizing addon configuration, not only addon directories
- handling bundled assets such as fonts and interface/material overrides

The key requirement is that the product must understand WoW installations as structured environments instead of treating them as flat folders.

## Goals

- Build a Rust-first, cross-platform core that works on Windows and macOS
- Separate engine logic from CLI and future GUI
- Support both individual addon operations and full setup bundles
- Make configuration sync safe through backup, preview, and rollback
- Define a bundle format we control instead of relying on undocumented third-party formats
- Keep reverse-engineering artifacts outside Git history

## Non-Goals

- Full GUI implementation in the first phase
- Deep support for every third-party launcher format on day one
- Live in-game integration
- Cloud account sync in the first phase

## Research Summary

The reference product behavior suggests that a practical WoW setup bundle is composed of multiple resource groups rather than one archive:

- unknown addon folders
- known addon packages
- account-level WTF data
- character-level WTF data
- fonts
- interface/material assets

The reference workflow also indicates that configuration sync is not a plain copy operation. Some Lua files require controlled text transformation when the target character or server differs from the source.

This confirms that `hearthsync` needs two layers:

1. file transport and archive management
2. domain-aware WoW configuration migration

## High-Level Product Model

### Installation Root

A WoW installation is modeled as:

- product root, for example `World of Warcraft`
- flavor root, for example `_retail_`, `_classic_era_`, `_classic_`
- content groups inside the flavor root

### Content Groups

The tool will explicitly model the following groups:

- `Interface/AddOns`
- `WTF/Config.wtf`
- `WTF/Account/<account>/SavedVariables`
- `WTF/Account/<account>/<server>/<character>`
- `Fonts`
- selected `Interface/*` subfolders outside `AddOns`

### Bundle Resource Groups

The portable bundle format will expose these resource groups:

- `addons`
- `wtf_common`
- `wtf_characters`
- `fonts`
- `interface_assets`
- `metadata`

This is intentionally close to real WoW structure while remaining explicit and portable.

## Architecture Decisions

### Decision 1: Use a layered Rust workspace

We will build the project as a small workspace-style architecture even if it starts in one crate.

Target modules:

- `core`: WoW domain model, sync engine, bundle engine
- `cli`: command parsing and human-readable output
- `gui`: future `egui` frontend

Rationale:

- keeps business logic reusable
- avoids CLI-specific assumptions leaking into the engine
- makes later GUI work cheaper

### Decision 2: Own the bundle format

We will define a first-party portable bundle format instead of directly copying third-party package conventions.

Proposed archive layout:

```text
bundle.zip
  manifest.toml
  addons/
  metadata/addons/lock.toml
  metadata/addons/indexes/
  wtf/common/
  wtf/characters/<character_key>/
  fonts/
  interface/
```

Rationale:

- easier to validate
- easier to version
- easier to migrate later
- keeps Windows and macOS behavior deterministic

### Decision 3: Store migration intent in the manifest

The manifest should not only describe files. It should also describe how to apply them.

Examples:

- source flavor
- supported target flavors
- account-level include and exclude rules
- character mapping mode
- whether Lua key rewriting is required
- backup and conflict strategy

Rationale:

- import behavior becomes reproducible
- preview and dry-run become possible
- future GUI can display the same intent cleanly

### Decision 4: Make backups mandatory before mutating config

Any command that modifies `WTF`, `Fonts`, or non-addon `Interface` content should create a backup checkpoint first.

Rationale:

- config sync is destructive by nature
- users will often apply someone else's setup to a live installation
- rollback is a core trust requirement

### Decision 5: Start with targeted Lua rewriting, not a full generic parser

Phase one will support a narrow, explicit rewrite engine for known migration cases:

- profile key replacement
- character and server identity replacement
- selected addon-specific string rewrites when clearly necessary

Rationale:

- lower implementation risk
- easier to test
- avoids pretending that all Lua saved variables can be safely transformed generically

### Decision 6: Treat WTF apply as selective account-targeted installation

The reference installer flow exposes `WTF` as a distinct install group and asks the user to choose
which local accounts should be overwritten before applying.

This implies two product rules:

- `WTF` install is not a single global toggle
- overwrite scope must be explicit at least at the local-account level

For `hearthsync`, the core must therefore support:

- bundle-level `WTF` presence detection
- local account discovery from the target installation
- explicit account selection before mutation
- optional character remapping inside each selected account

Rationale:

- matches user expectations from existing tools
- reduces accidental destructive writes
- cleanly separates common account config from character-targeted migration

### Decision 7: Keep per-group mutation policies instead of one global replace mode

The reference flow uses different defaults per resource group:

- known addons: can clear local content before install
- unknown addons: can clear local content before install
- `WTF`: does not use the same clear-local default
- materials and fonts: can clear local content before install

For `hearthsync`, apply behavior should be modeled per group, for example:

- `addons.replace_mode`
- `wtf.account_merge_mode`
- `fonts.replace_mode`
- `interface_assets.replace_mode`

Rationale:

- resource groups have different safety characteristics
- `WTF` needs merge or targeted replacement semantics, not a flat delete-and-copy
- future GUI can expose the same behavior without inventing frontend-only rules

### Decision 8: Abstract migration helpers behind engine capabilities

The reference application appears to detect and use a local diff/migration helper when available,
and falls back to another path when it is not.

For `hearthsync`, helper-assisted migration must be modeled as an optional capability behind the
same engine contract:

- native Rust migration path is the baseline
- optional external helper integration may be added later
- progress reporting and failure handling must not depend on a specific helper

Rationale:

- keeps the core cross-platform and deterministic
- avoids coupling the product to a proprietary local helper
- still leaves room for future acceleration paths on platforms where helper tooling exists

## Proposed Repository Layout

```text
docs/
  workstreams/
    wow-addon-sync-cli/
      design.md
      todo.md
      milestones.md
src/
  main.rs
```

Target source layout after the first development pass:

```text
src/
  main.rs
  cli/
  core/
    addon/
    backup/
    bundle/
    install/
    lua_patch/
    manifest/
    platform/
    sync/
    wow/
```

If the codebase grows, these modules can be promoted into separate crates.

## Domain Model

### WoWInstallation

Represents one detected installation.

Fields:

- platform
- root_path
- flavor
- executable_path
- addon_dir
- wtf_dir
- fonts_dir
- interface_dir

### BundleManifest

Represents the portable package description.

Suggested fields:

- bundle_version
- created_at
- source_platform
- source_flavor
- source_install_label
- bundle_name
- description
- resources
- mapping_rules
- apply_defaults

### CharacterSelector

Represents a source or target character identity.

Fields:

- account
- server
- character

### ApplyPlan

Represents the resolved set of changes before execution.

Fields:

- files_to_add
- files_to_replace
- files_to_skip
- rewrites
- selected_target_accounts
- group_policies
- helper_strategy

## Reference UX Findings

The targeted review of NewBeeBox indicates the following concrete behaviors that are worth copying:

- install UI is grouped by `known addons`, `unknown addons`, `WTF`, `materials`, and `fonts`
- `WTF` is described as local game account and character configuration data
- `WTF` does not share the same clear-local default as addons, materials, and fonts
- the installer exposes `select overwrite local account {count}/{total}` for `WTF`
- the internal install path logs `selectLocalWtfList`, which suggests account-level selection is
  passed into the migration task
- the app appears to distinguish common config download from role config download before apply
- the app appears to detect an optional diff-sync helper before choosing the installation path

These findings strengthen the case for a two-stage `WTF` apply model in `hearthsync`:

1. resolve selected target accounts and character mappings
2. execute copy and Lua rewrite steps under an explicit apply plan
- backup_id
- warnings

## Bundle Manifest Proposal

`manifest.toml` example:

```toml
bundle_version = 1
bundle_name = "Rurutia Retail Setup"
source_platform = "windows"
source_flavor = "retail"

[apply_defaults]
backup_before_apply = true
replace_addons = true
replace_fonts = true
replace_interface_assets = true
merge_wtf_common = true
merge_wtf_character = true

[mapping_rules]
character_mapping = "prompt"
rewrite_profile_keys = true
rewrite_character_identity = true

[resources]
addons = true
addon_lock = true
addon_indexes = ["addon-index.toml"]
wtf_common = true
wtf_characters = true
fonts = true
interface_assets = true
```

Addon metadata is stored as sidecar data instead of overwriting the target machine's active addon registry:

- `metadata/addons/lock.toml` contains the refreshed addon lock from the source installation
- `metadata/addons/indexes/*.toml` contains curated addon indexes referenced by the manifest
- unpack writes these files to `Interface/AddOns/.hearthsync/bundles/<bundle-id>/...`
- users can then run `addon lock plan/apply --file <sidecar-lock>` explicitly

## CLI Surface Proposal

### Installation and Discovery

- `hearthsync scan`
- `hearthsync inspect --install <path>`
- `hearthsync doctor --install <path>`

### Addon Operations

- `hearthsync addon index inspect`
- `hearthsync addon index install`
- `hearthsync addon index update`
- `hearthsync addon lock inspect`
- `hearthsync addon lock write`
- `hearthsync addon lock verify`
- `hearthsync addon lock diff`
- `hearthsync addon lock plan`
- `hearthsync addon lock apply`
- `hearthsync addon search`
- `hearthsync addon list`
- `hearthsync addon install <source>`
- `hearthsync addon update [name]`
- `hearthsync addon remove <name>`

### Addon Source Model v1

The first addon-management implementation uses a provider-neutral source abstraction:

- local zip archive path
- direct `http/https` zip archive URL
- `CurseForge` shortcut source in the form `curseforge:modId[@fileId]`
- `GitHub Releases` shortcut source in the form `github:owner/repo[@tag][#asset.zip]`

The CLI installs from a staged archive extraction flow instead of writing from the archive directly.
Addon roots are detected from `.toc` files whose basename matches the addon directory name.
This allows archives with an extra wrapper directory to be normalized into `Interface/AddOns/<addon>`.

Installed addon packages are tracked per WoW installation in:

- `Interface/AddOns/.hearthsync/addons.toml`
- `Interface/AddOns/.hearthsync/lock.toml`

This registry stores:

- source reference
- package identifier
- installed addon directory names
- optional `.toc` metadata such as title and version
- install and update timestamps

The derived lock file stores:

- the tracked package source reference
- optional metadata projected from custom addon indexes
- addon directory names and recorded addon metadata
- install and update timestamps
- a deterministic `content_sha256` fingerprint of the installed files for drift detection and cross-machine verification

`addon update` reuses the recorded source reference, refreshes the tracked addon directories, and creates an AddOns backup before mutation.
`addon remove` resolves a tracked package by package id or addon directory name, removes its recorded addon directories, and deletes the local receipt file when the registry becomes empty.
Successful addon mutations refresh the derived lock automatically, while `addon lock write` can rebuild it from the registry on demand.
`addon lock verify` compares a lock file against the current installation and reports content-hash drift, missing tracked addon directories, unexpected tracked packages, and untracked addon directories.
`addon lock diff` compares two lock files directly for cross-machine checks.
`addon lock plan` translates the lock into install/update/remove actions, while `addon lock apply` executes that plan against the current installation.

`CurseForge` is the second real provider in the system.
Resolution rules in the current version:

- `curseforge:modId` selects the newest available `.zip` file returned by the official mod-files API
- `curseforge:modId@fileId` pins one exact file
- the provider requires environment variable `HEARTHSYNC_CURSEFORGE_API_KEY`
- current v1 filters by the target WoW flavor through official CurseForge version-type metadata
- explicit `fileId` is still the safest path when one flavor exposes multiple parallel builds
- `addon search` uses the official search API and returns an install hint such as `curseforge:12345@67890`

`GitHub Releases` is the first metadata-backed provider in the system.
Resolution rules in the current version:

- `github:owner/repo` resolves the latest release
- `github:owner/repo@tag` resolves a specific release tag
- `github:owner/repo#asset.zip` selects an explicit asset when a release contains multiple zip files
- if a release contains multiple zip assets and no asset is specified, the command fails with a clear disambiguation error

### Custom Addon Index v1

Custom indexes are local TOML files for curated addon lists.
They are intentionally provider-neutral and store source references using the same serialized `AddonSourceRef` model as the addon registry.

Example:

```toml
schema_version = 1
name = "Example Raid UI"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "5.0.0"
source = { kind = "github_release", owner = "WeakAuras", repo = "WeakAuras2", tag = "5.0.0", asset_name = "WeakAuras-5.0.0.zip" }
supported_flavors = ["retail"]
```

Current commands:

- `addon index inspect --file <index.toml>`
- `addon index install --file <index.toml> --name <id-or-name>`
- `addon index update --file <index.toml> [--name <id-or-name>]`
- `addon lock inspect --install <wow-path>`
- `addon lock write --install <wow-path>`
- `addon lock verify --install <wow-path> [--file <lock.toml>]`
- `addon lock diff --left-file <lock-a.toml> --right-file <lock-b.toml>`
- `addon lock plan --install <wow-path> [--file <lock.toml>]`
- `addon lock apply --install <wow-path> [--file <lock.toml>] [--replace-existing]`

The first version validates package ids, duplicate ids, source references, and optional flavor compatibility before delegating to the existing install/update pipeline.
When index-based installs or updates succeed, curated package metadata is carried into the lock file so later GUI flows can present stable names, pinned versions, and source hashes without reopening the original index.
Lock comparison prefers index package identity when available, then falls back to the tracked package id. Generated and install timestamps are intentionally ignored by diff/verify because they are expected to differ across machines.
Sync planning and apply prefer index identity when it exists, then fall back to the normalized addon directory set. This avoids false add/remove churn when two machines install the same addon from archives with different file names.

### Bundle Operations

- `hearthsync bundle pack`
- `hearthsync bundle inspect <bundle>`
- `hearthsync bundle apply <bundle>`

### Config Operations

- `hearthsync config export`
- `hearthsync config sync`
- `hearthsync config preview`

### Safety Operations

- `hearthsync backup create`
- `hearthsync backup list`
- `hearthsync backup restore <id>`

`backup list` reads backup metadata from the configured backup directory and reports:

- backup id
- created time
- optional label
- flavor and resource groups
- archive path and size

`backup restore` supports two lookup modes:

- restore by `backup_id` from the backup directory
- restore by explicit archive path

## Packing Workflow

`bundle pack` should support:

- selecting one installation
- selecting content groups to include
- embedding the source addon lock and curated addon index files as metadata
- selecting one or more characters
- include and exclude globs
- optional pruning of cache and backup files

The command should produce:

- a validated `manifest.toml`
- a zip archive with normalized paths
- a summary report of included files and total size

## Apply Workflow

`bundle apply` should follow this sequence:

1. detect target installation
2. validate target flavor compatibility
3. inspect manifest
4. resolve target character mapping
5. build an `ApplyPlan`
6. create a backup checkpoint
7. extract bundle into a staging directory
8. apply resource groups in deterministic order
9. run targeted Lua rewrites
10. emit a machine-readable and human-readable report

Recommended apply order:

1. addons
2. interface assets
3. fonts
4. common WTF
5. character WTF
6. post-apply rewrites
7. optional explicit addon lock plan/apply from embedded sidecar metadata

## Cross-Platform Concerns

### Path Semantics

- Windows is case-insensitive by default
- macOS may be case-insensitive or case-sensitive
- archive paths must be normalized to `/`

### Filename Encoding

Some zip archives may contain non-UTF-8 names.
The engine must normalize names during extraction and should preserve a clear error if the encoding cannot be resolved safely.

### WoW Flavor Compatibility

Retail and Classic variants must be treated as separate targets.
The manifest should declare its source flavor and optionally its compatible flavors.

### Character Identity Portability

A Windows source bundle may be applied to a macOS installation, but account, server, and character mapping still needs explicit user intent.
The tool must never silently assume the first available target character.

## Reverse Engineering and Local Research Policy

Temporary extracted reference files may be stored under:

- `targets/research/`

This directory is intentionally ignored by Git.
Research artifacts should be summarized in docs rather than committed as extracted binaries or vendor code.

## Implementation Strategy

### Phase 1

- establish CLI skeleton
- implement installation scanning
- implement bundle manifest model
- implement backup checkpoint creation

### Phase 2

- implement bundle packing
- implement bundle inspection
- implement staged extraction and apply preview

### Phase 3

- implement addon install and update workflows
- implement targeted config sync and Lua rewrites

### Phase 4

- harden rollback, reporting, and test coverage
- prepare stable core interfaces for `egui`

## Open Questions

- Which addon metadata sources should be supported first
- How aggressive should addon directory replacement be by default
- Which WoW cache files should always be excluded
- Which addon-specific rewrites deserve first-class support beyond generic profile key mapping

## Immediate Next Build Target

The first implementation target after documentation is:

1. scaffold the CLI
2. add installation scanning
3. add bundle manifest types
4. add backup checkpoint creation

This creates the minimum safe base for later pack and apply flows.
