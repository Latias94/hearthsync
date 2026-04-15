# HearthSync

HearthSync is a cross-platform command-line tool for managing and synchronizing World of Warcraft addon setups.
It focuses on safe addon installation, portable UI bundles, backup/restore workflows, and future reuse from an `egui` desktop UI.

> Status: early alpha. The CLI and core models are actively evolving.

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
- Search CurseForge projects and install from local zip archives, direct `http(s)` zip URLs, GitHub Releases, or CurseForge project references.

## Installation

Build from source:

```powershell
cargo build --release
```

Run the CLI during development:

```powershell
cargo run -- --help
```

## Basic Usage

Scan known installation locations:

```powershell
cargo run -- scan
```

Inspect a WoW installation:

```powershell
cargo run -- inspect --install "E:\Games\World of Warcraft" --flavor retail
```

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

Install from supported sources:

```powershell
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source .\WeakAuras.zip
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source "github:owner/repo#addon.zip"
cargo run -- addon install --install "E:\Games\World of Warcraft" --flavor retail --source "curseforge:12345@67890"
```

Create a portable bundle:

```powershell
cargo run -- manifest example > manifest.toml
cargo run -- bundle pack --install "E:\Games\World of Warcraft" --flavor retail --manifest .\manifest.toml --output .\my-ui.zip
```

Preview and unpack a bundle:

```powershell
cargo run -- bundle plan --bundle .\my-ui.zip --install "E:\Games\World of Warcraft" --flavor retail
cargo run -- bundle unpack --bundle .\my-ui.zip --install "E:\Games\World of Warcraft" --flavor retail --dry-run
```

## Addon Source Formats

HearthSync currently supports:

- Local zip archives: `.\Addon.zip`
- Direct zip URLs: `https://example.com/Addon.zip`
- GitHub Releases: `github:owner/repo[@tag][#asset.zip]`
- CurseForge: `curseforge:modId[@fileId]`

CurseForge access requires an official CurseForge REST API key in `HEARTHSYNC_CURSEFORGE_API_KEY`.
HearthSync does not re-host CurseForge files; it resolves metadata and downloads through the provider API.

## Safety Model

Mutating workflows are designed around:

- staging archive extraction before writes
- dry-run plans where available
- automatic backup creation before apply/update/remove operations
- restore support for backup archives
- path normalization and zip-slip protection
- account/character mapping instead of blindly overwriting `WTF`

## Documentation

Workstream docs live under:

- `docs/workstreams/wow-addon-sync-cli/design.md`
- `docs/workstreams/wow-addon-sync-cli/todo.md`
- `docs/workstreams/wow-addon-sync-cli/milestones.md`

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

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.
