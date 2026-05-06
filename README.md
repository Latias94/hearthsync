# HearthSync

HearthSync is a cross-platform command-line tool for managing and synchronizing World of Warcraft addon setups.
It focuses on safe addon installation, portable UI bundles, backup/restore workflows, and future reuse from an `egui` desktop UI.

> Status: early alpha. The CLI and core models are actively evolving.
> See [Product Readiness](docs/product-readiness.md) for the current technical-preview boundary and
> public-release checklist.

## Goals

- Manage WoW addons on Windows and macOS.
- Package complete UI setups, including addons, `WTF` configuration, fonts, textures, and interface assets.
- Safely migrate addon configurations between accounts, realms, and characters.
- Provide dry-run planning, automatic backups, and rollback-friendly workflows before mutating a WoW installation.
- Keep provider downloads attributed to their original sources and avoid re-hosting third-party addon files.

## Current Features

- Detect and inspect local WoW installations.
- Create, list, and restore backup archives.
- Pack portable UI bundles from a WoW installation.
- Inspect, plan, and unpack bundles into another WoW installation.
- Apply character/account remapping for `WTF` configuration.
- Rewrite common Lua profile keys and character identity strings during migration.
- Install, update, remove, and list tracked addon packages.
- Search CurseForge and Tukui projects and install from local zip archives, direct `http(s)` zip URLs, GitHub Releases, CurseForge, Wago, or Tukui references.

## Installation

Build from source:

```powershell
cargo build --release
```

Run the CLI during development:

```powershell
cargo run -- --help
```

## Network And Provider Credentials

HearthSync keeps shared discovery catalog-backed by default and only calls live provider search APIs
when a provider-scoped search is requested. Installs and updates still contact the original provider
when resolving the selected source.

Useful environment variables:

```powershell
$env:HEARTHSYNC_CURSEFORGE_API_KEY = "<your official CurseForge REST API key>"
$env:HEARTHSYNC_GITHUB_TOKEN = "<optional GitHub token>"
$env:HTTPS_PROXY = "http://127.0.0.1:10809"
```

Notes:

- CurseForge API access requires `HEARTHSYNC_CURSEFORGE_API_KEY`.
- GitHub resolution works anonymously for light use, but heavier install/update/validation flows
  should use `HEARTHSYNC_GITHUB_TOKEN` or `GITHUB_TOKEN`.
- Standard `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and `NO_PROXY` variables are honored by the
  HTTP client stack.
- Live provider search results are cached in-process for 300 seconds by default. Use
  `--addon-search-cache-ttl-secs 0` to disable that cache for debugging, or set another TTL globally
  or through `settings set`.

## Basic Usage

### Safe First Run

Start with read-only commands. These commands inspect local state and runtime capability without
writing to the WoW installation:

```powershell
cargo run -- runtime
cargo run -- scan
cargo run -- inspect --install "E:\Games\World of Warcraft" --flavor retail
```

Search the in-tree community addon catalog without calling live provider search APIs:

```powershell
cargo run -- addon index search --file .\catalog\community-addon-index.toml --query WeakAuras
```

Inspect a config package before applying it:

```powershell
cargo run -- config inspect --source .\AuthorUI
```

Use `plan` or `--dry-run` before any apply/install workflow that can mutate an installation.

Scan known installation locations:

```powershell
cargo run -- scan
```

Inspect current runtime diagnostics and stable capability projection:

```powershell
cargo run -- runtime
```

Inspect runtime diagnostics for one concrete installation, including exact managed addon state
paths:

```powershell
cargo run -- runtime --install "E:\Games\World of Warcraft" --flavor retail
```

Inspect a WoW installation:

```powershell
cargo run -- inspect --install "E:\Games\World of Warcraft" --flavor retail
```

HearthSync keeps scan-only inspection separate from tracked addon management.
Commands such as `scan` and `inspect` do not need to create managed addon state first.
Tracked addon workflows still use persistent managed state, which defaults to platform app-data and
can be switched to portable sidecar mode explicitly:

```powershell
cargo run -- --addon-state-storage sidecar addon list --install "E:\Games\World of Warcraft" --flavor retail
```

In the default app-data backend, neither scan-only commands nor normal tracked-addon commands need
to create `Interface/AddOns/.hearthsync`.
That sidecar path appears only when you explicitly choose `--addon-state-storage sidecar` or when
bundle metadata is unpacked on purpose under `.hearthsync/bundles/...`.

Create and list backups:

```powershell
cargo run -- backup create --install "E:\Games\World of Warcraft" --flavor retail
cargo run -- backup list
```

Restore a backup by archive path or backup id:

```powershell
cargo run -- backup restore --install "E:\Games\World of Warcraft" --flavor retail --archive .\backup.zip
cargo run -- backup restore --install "E:\Games\World of Warcraft" --flavor retail --id backup-retail-example
```

Search addons through CurseForge:

```powershell
$env:HEARTHSYNC_CURSEFORGE_API_KEY = "<your official CurseForge REST API key>"
cargo run -- addon search --install "E:\Games\World of Warcraft" --flavor retail --query WeakAuras --limit 5
```

Search Tukui/ElvUI metadata:

```powershell
cargo run -- addon search --install "E:\Games\World of Warcraft" --flavor retail --provider tukui --query ElvUI --limit 5
```

Install from supported sources:

```powershell
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source .\WeakAuras.zip
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source "github:owner/repo#addon.zip"
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source "curseforge:12345@67890"
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source "wago:qv63A7Gb"
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source "tukui:elvui"
```

Install from a custom addon index:

```powershell
cargo run -- addon index inspect --file .\addon-index.toml
cargo run -- addon index install --install "E:\Games\World of Warcraft" --flavor retail --file .\addon-index.toml --name WeakAuras
cargo run -- addon index update --install "E:\Games\World of Warcraft" --flavor retail --file .\addon-index.toml
cargo run -- addon lock inspect --install "E:\Games\World of Warcraft" --flavor retail
cargo run -- addon lock write --install "E:\Games\World of Warcraft" --flavor retail
```

Create a portable bundle:

```powershell
cargo run -- manifest example > manifest.toml
cargo run -- bundle pack --install "E:\Games\World of Warcraft" --flavor retail --manifest .\manifest.toml --output .\my-ui.zip
```

Bundle manifests can also include addon state metadata:

```toml
[resources]
addon_lock = true
addon_indexes = [".\\addon-index.toml"]
```

When present, HearthSync embeds the refreshed addon lock at `metadata/addons/lock.toml`, package source archives under `metadata/addons/sources/`, a source map at `metadata/addons/sources.toml`, and addon indexes under `metadata/addons/indexes/`.
Unpacking writes those files as sidecar metadata under `Interface/AddOns/.hearthsync/bundles/<bundle-id>/` so users can run `addon lock plan/apply --file <sidecar-lock>` explicitly without silently replacing local addon tracking; the sidecar `sources.toml` is detected automatically.
For convenience, bundles can also be used directly as addon sync inputs:

```powershell
cargo run -- bundle addon-plan --bundle .\my-ui.zip --install "E:\Games\World of Warcraft" --flavor retail
cargo run -- bundle addon-apply --bundle .\my-ui.zip --install "E:\Games\World of Warcraft" --flavor retail --backup-output .\backups --replace-existing
```

These commands read `metadata/addons/lock.toml` from the bundle without unpacking sidecar files first.
If the lock points to a source-machine local archive path, the direct bundle commands use the embedded package archive as a transport fallback, while keeping the original source reference in the target registry for attribution and future provider-based updates.
They also honor the global `--addon-state-storage app-data|sidecar` runtime setting for the
target installation's managed addon state; only unpacked bundle metadata continues to land under
`Interface/AddOns/.hearthsync/bundles/<bundle-id>/`.

Preview and unpack a bundle:

```powershell
cargo run -- bundle plan --bundle .\my-ui.zip --install "E:\Games\World of Warcraft" --flavor retail
cargo run -- bundle unpack --bundle .\my-ui.zip --install "E:\Games\World of Warcraft" --flavor retail --dry-run
```

## Config Package Workflow

Config packages are author-style UI setup folders or zips that may contain `Interface/AddOns`,
`WTF`, `Fonts`, and additional `Interface` resources. The `config` commands are a product-facing
alias over the same safe external-package and bundle pipeline used by portable bundles.

Inspect a package before sharing or applying it:

```powershell
cargo run -- config inspect --source .\AuthorUI
```

Export a reusable HearthSync bundle from a config package:

```powershell
cargo run -- config export --source .\AuthorUI --source-flavor retail --source-platform windows --output .\author-ui.hearthsync.zip
```

Public sharing is blocked until review-required WTF and SavedVariables risks are explicitly
accepted:

```powershell
cargo run -- config export --source .\AuthorUI --source-flavor retail --output .\author-ui-public.hearthsync.zip --sharing-mode public --allow-public-sharing-risks
```

Preview and apply a config package with account, realm, and character mapping:

```powershell
cargo run -- config plan --source .\AuthorUI --source-flavor retail --source-platform windows --install "E:\Games\World of Warcraft" --flavor retail --target-account ACCOUNT --target-server Illidan --target-character Examplemage
cargo run -- config apply --source .\AuthorUI --source-flavor retail --source-platform windows --install "E:\Games\World of Warcraft" --flavor retail --backup-output .\backups --target-account ACCOUNT --target-server Illidan --target-character Examplemage
```

## Addon Source Formats

HearthSync currently supports:

- Local zip archives: `.\Addon.zip`
- Direct zip URLs: `https://example.com/Addon.zip`
- GitHub Releases: `github:owner/repo[@tag][#asset.zip]`
- CurseForge: `curseforge:modId[@fileId]`
- Wago addons: `wago:projectId[@releaseId]`
- Tukui addons: `tukui:slug[@current-version]`

CurseForge access requires an official CurseForge REST API key in `HEARTHSYNC_CURSEFORGE_API_KEY`.
HearthSync does not re-host CurseForge files; it resolves metadata and downloads through the provider API.
Tukui sources use the official latest addon API; the optional version acts as a current-version guard,
not a historical release replay guarantee.

## Custom Addon Index

Custom indexes let users, guilds, or maintainers publish a curated list of addon sources without depending on a third-party search API.

Example:

```toml
schema_version = 1
name = "Example Raid UI"
description = "Pinned addon sources for a raid team"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "5.0.0"
source = { kind = "github_release", owner = "WeakAuras", repo = "WeakAuras2", tag = "5.0.0", asset_name = "WeakAuras-5.0.0.zip" }
website_url = "https://github.com/WeakAuras/WeakAuras2"
addon_directories = ["WeakAuras", "WeakAurasOptions"]
supported_flavors = ["retail"]
```

Supported `source.kind` values are the same source references used by the install command:

- `local_archive`
- `http_archive`
- `github_release`
- `curseforge_mod`
- `wago_addon`
- `tukui_addon`

The repository also ships a starter community catalog at `catalog/community-addon-index.toml`
plus a governance overlay at `catalog/community-addon-index.governance.json`.
The TOML file follows the same index schema, so you can inspect it directly:

```powershell
cargo run -- addon index inspect --file .\catalog\community-addon-index.toml
cargo run -- addon index search --file .\catalog\community-addon-index.toml --query ElvUI
```

For repeatable read-only checks, use `scripts/catalog-readonly-validation.ps1`.
It validates the governance overlay, alias search, and live provider dry-run probes as well as the
base index.

## Addon Lock File

HearthSync maintains a derived addon lock file in the managed addon state backend.
By default that backend lives under platform app-data keyed by installation identity.
If you switch to portable sidecar mode with `--addon-state-storage sidecar`, the lock returns to
`Interface/AddOns/.hearthsync/lock.toml`.
Use `cargo run -- runtime --install "<wow-path>" --flavor <flavor>` to inspect the exact resolved
managed-state paths for a specific installation.
Staying on the default app-data backend means tracked addon state does not need to create
`Interface/AddOns/.hearthsync` just to hold the registry, lock, or policy files.

The lock file is generated from the tracked addon registry and records:

- tracked package id and source reference
- optional curated metadata from a custom addon index
- installed addon directories and `.toc` metadata
- install and update timestamps
- a deterministic `content_sha256` fingerprint of the installed addon files

It is refreshed automatically after successful addon install, update, remove, and index workflows.
You can also inspect or regenerate it manually with:

- `hearthsync addon lock inspect --install <wow-path> [--flavor <flavor>]`
- `hearthsync addon lock write --install <wow-path> [--flavor <flavor>]`
- `hearthsync addon lock verify --install <wow-path> [--flavor <flavor>] [--file <lock.toml>]`
- `hearthsync addon lock diff --left-file <lock-a.toml> --right-file <lock-b.toml>`
- `hearthsync addon lock plan --install <wow-path> [--flavor <flavor>] [--file <lock.toml>]`
- `hearthsync addon lock apply --install <wow-path> [--flavor <flavor>] [--file <lock.toml>] [--backup-output <dir>]`

`verify` compares the lock against the current installation and reports changed package hashes, missing tracked addon directories, unexpected tracked packages, and untracked addon directories.
`diff` compares two lock files, which is useful when checking whether a Windows and macOS install are carrying the same addon set.
`plan` translates the lock into concrete install/update/remove actions against the current installation.
`apply` executes that sync plan; when untracked addon directories conflict with the desired lock, pass `--replace-existing` explicitly.

## Safety Model

Mutating workflows are designed around:

- staging archive extraction before writes
- dry-run plans where available
- automatic backup creation before apply/update/remove operations
- automatic addon lock refresh after successful addon mutations
- restore support for backup archives
- path normalization and zip-slip protection
- account/character mapping instead of blindly overwriting `WTF`

## Documentation

Workstream docs live under:

- `docs/workstreams/wow-addon-sync-cli/design.md`
- `docs/workstreams/wow-addon-sync-cli/todo.md`
- `docs/workstreams/wow-addon-sync-cli/milestones.md`

Repository-owned addon metadata lives in:

- `catalog/community-addon-index.toml`
- `catalog/community-addon-index.governance.json`
- `catalog/README.md`

## Development

Format code:

```powershell
cargo fmt
```

Run tests:

```powershell
cargo nextest run
```

If `cargo nextest` is not installed, use:

```powershell
cargo test
```

## Distribution and Mod Author Respect

HearthSync is intended to be a user-side sync and management tool, not a third-party mod redistribution platform.
Provider-hosted mods should remain attributed to their original projects and downloaded through official provider APIs or user-provided sources.
Bundles that include `metadata/addons/sources/` contain addon package files and should be treated as personal migration backups unless the mod authors' licenses and platform terms allow redistribution.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.
