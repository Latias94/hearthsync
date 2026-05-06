# User Workflows

This guide gives opinionated command recipes for the current CLI technical preview.

All examples assume PowerShell and a retail installation at:

```powershell
$Install = "E:\Games\World of Warcraft"
$Flavor = "retail"
```

Prefer read-only commands first:

- `runtime`
- `scan`
- `inspect`
- `addon index search`
- `config inspect`
- `plan`
- `--dry-run`

## 1. Adopt Existing Addons

Use this when you already have manually installed addon folders under `Interface/AddOns` and want
HearthSync to track them without guessing a remote source.

Inspect the installation first:

```powershell
cargo run -- inspect --install $Install --flavor $Flavor
cargo run -- addon list --install $Install --flavor $Flavor
```

Preview adoption for one addon directory:

```powershell
cargo run -- addon adopt --install $Install --flavor $Flavor --addon WeakAuras --dry-run
```

Adopt it. HearthSync creates a snapshot archive and records that archive as the tracked source:

```powershell
cargo run -- addon adopt --install $Install --flavor $Flavor --addon WeakAuras
```

For multi-folder addons, use one explicit package id:

```powershell
cargo run -- addon adopt --install $Install --flavor $Flavor --package-id weakauras --addon WeakAuras --addon WeakAurasOptions --dry-run
cargo run -- addon adopt --install $Install --flavor $Flavor --package-id weakauras --addon WeakAuras --addon WeakAurasOptions
```

If the package exists in the community catalog, attach the curated source metadata without
reinstalling files:

```powershell
cargo run -- addon index relink --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --name WeakAuras --target weakauras --dry-run
cargo run -- addon index relink --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --name WeakAuras --target weakauras
```

Write a lock after the tracked state looks correct:

```powershell
cargo run -- addon lock write --install $Install --flavor $Flavor
cargo run -- addon lock inspect --install $Install --flavor $Flavor
```

## 2. Install From The Community Catalog

Use this when you want curated source metadata without live provider search.

Inspect and search the in-tree catalog:

```powershell
cargo run -- addon index inspect --file .\catalog\community-addon-index.toml
cargo run -- addon index search --file .\catalog\community-addon-index.toml --query WeakAuras
```

Preview install:

```powershell
cargo run -- addon index install --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --name WeakAuras --dry-run
```

Install with an explicit backup directory:

```powershell
cargo run -- addon index install --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --name WeakAuras --backup-output .\backups
```

Update tracked catalog packages:

```powershell
cargo run -- addon index update --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --dry-run
cargo run -- addon index update --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --backup-output .\backups
```

If provider-backed resolution fails because of network access or quota, configure credentials or
proxy environment variables before retrying. See `README.md` for the supported environment
variables.

## 3. Share Your UI Setup

Use this when you want to export your current UI setup for another machine or another user.

Start with inspection. For a live WoW install, point at the flavor root:

```powershell
cargo run -- config inspect --source "$Install\_retail_"
```

Create a private bundle for your own migration:

```powershell
cargo run -- config export --source "$Install\_retail_" --source-flavor $Flavor --source-platform windows --output .\my-ui-private.hearthsync.zip
```

Public sharing is stricter because `WTF` and SavedVariables can include account names, character
names, addon tokens, chat/history data, or other user-specific values. Either keep the bundle
private or exclude sensitive WTF scopes before public export:

```powershell
cargo run -- config export --source "$Install\_retail_" --source-flavor $Flavor --source-platform windows --sharing-mode public --exclude-wtf-scope account-saved-variables --output .\my-ui-public.hearthsync.zip
```

If you intentionally accept the public-sharing review risks, make that explicit:

```powershell
cargo run -- config export --source "$Install\_retail_" --source-flavor $Flavor --source-platform windows --sharing-mode public --allow-public-sharing-risks --output .\my-ui-public-reviewed.hearthsync.zip
```

Verify the exported bundle before sharing it:

```powershell
cargo run -- bundle inspect --bundle .\my-ui-private.hearthsync.zip
```

## 4. Apply An Author Package

Use this when an addon author or UI author provides a zip/folder that contains `Interface/AddOns`,
`WTF`, `Fonts`, or `Interface` assets.

Inspect first:

```powershell
cargo run -- config inspect --source .\AuthorUI
```

Build a plan with explicit target identity mapping:

```powershell
cargo run -- config plan --source .\AuthorUI --source-flavor $Flavor --source-platform windows --install $Install --flavor $Flavor --target-account ACCOUNT --target-server Illidan --target-character Examplemage
```

Dry-run apply before writing:

```powershell
cargo run -- config apply --source .\AuthorUI --source-flavor $Flavor --source-platform windows --install $Install --flavor $Flavor --target-account ACCOUNT --target-server Illidan --target-character Examplemage --dry-run --backup-output .\backups
```

Apply only after reviewing the plan:

```powershell
cargo run -- config apply --source .\AuthorUI --source-flavor $Flavor --source-platform windows --install $Install --flavor $Flavor --target-account ACCOUNT --target-server Illidan --target-character Examplemage --backup-output .\backups
```

For account-wide packages, add `--select-account ACCOUNT` or `--all-accounts` deliberately. Avoid
using `--all-accounts` for public or untrusted packages unless the plan output is exactly what you
expect.

## Recovery Commands

List available backups:

```powershell
cargo run -- backup list
```

Restore by archive path or backup id:

```powershell
cargo run -- backup restore --install $Install --flavor $Flavor --archive .\backups\backup.zip
cargo run -- backup restore --install $Install --flavor $Flavor --id backup-retail-example
```

Verify tracked addon state after recovery:

```powershell
cargo run -- addon lock verify --install $Install --flavor $Flavor
```

## When To Stop

Stop and inspect before applying if:

- the plan targets an unexpected account, realm, or character;
- a config package reports public-sharing review risks that you do not understand;
- live provider resolution fails repeatedly under a proxy or quota limit;
- an addon package expands to unexpected addon directories;
- an update wants to replace untracked addon directories.

The current CLI is a technical preview. It is designed to fail closed, but it still expects the
operator to review plans before mutating a live game tree.
