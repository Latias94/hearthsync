# HearthSync Catalog

This directory holds the repository-owned addon catalog.

It is metadata-only:

- no addon zip archives
- no user registry state
- no backup or restore payloads
- no hidden mirror of provider archives

Contribution workflow and review expectations live in
[`docs/catalog-contribution.md`](../docs/catalog-contribution.md).

## Current Shape

The catalog is split into two files:

- [`community-addon-index.toml`](./community-addon-index.toml) is the installable/searchable index
  and keeps the existing addon-index schema.
- [`community-addon-index.governance.json`](./community-addon-index.governance.json) is the
  editorial overlay for aliases, upstream hosts, source attribution, maintainer ownership,
  confidence, status, and last verification time.

That split keeps install-time schema conservative while still giving us a machine-readable place
for discovery and governance metadata.

In practice, each package record in the TOML index is a curated source mapping:

- `id` is the stable package key inside HearthSync.
- `name` is the human-facing addon name.
- `version` is the last verified upstream version snapshot, not a promise of install-time replay.
- `source` is the canonical provider identity that install/update resolves at request time.
- `source_url` is the best machine-followable upstream landing page for the source.
- `website_url` is the human landing page.
- `match_package_ids` preserve historical package-id continuity across source drift.
- `addon_directories` enumerate the exact folders the archive should expand into.
- `supported_flavors` list the WoW flavors the package is known to support.

In the governance overlay, each package record adds:

- `aliases` for user-facing discovery terms
- `upstream_hosts` for canonical host labels such as `github`, `wago`, or `tukui`
- `source_attribution` for the upstream identity note
- `maintainer` for catalog ownership
- `status` for active or lifecycle state
- `confidence` for how strong the mapping is
- `last_verified_at` for read-only validation freshness
- `notes` for maintainer-facing context

## Source Policy

- Prefer upstream-owned source references.
- Keep GitHub entries pointed at the owning repository, not a mirror or a pinned release asset.
- Keep Wago entries pointed at the project id, and only pin release ids when a source must be made reproducible.
- Keep Tukui entries pointed at the official slug and optional current-version guard only.
- Do not store downloaded archives, extracted payloads, or user registry state.
- Do not store anything that would make the catalog a hidden redistribution layer.

## Why Split The Catalog This Way

- The current CLI already validates this shape.
- Existing exact-hint validation tells us whether a catalog entry can be matched safely.
- The index can seed the product without locking us into a second install schema too early.
- The provider registry already knows how to consume the same source references.
- The governance overlay can evolve independently without forcing addon-install callers to learn
  editorial metadata.

If the governance overlay later needs richer editorial metadata, keep it outside the addon-install
schema so the install/update path stays conservative.

## Validation

The catalog index file can be validated with the existing addon-index inspection command:

```powershell
cargo run -- addon index inspect --file .\catalog\community-addon-index.toml
```

That checks schema validity, source reference validity, and addon-directory portability.

For repeatable local validation, use [`scripts/catalog-readonly-validation.ps1`](../scripts/catalog-readonly-validation.ps1).
It verifies schema health, governance overlay coverage, alias search, and live provider dry-run
probes without mutating addon state.

Those probes verify source refs against live upstream endpoints without mutating local addon state.

The catalog is also searchable through the local addon-index search entry:

```powershell
cargo run -- addon index search --file .\catalog\community-addon-index.toml --query ElvUI
```

Alias-driven discovery also works when the governance overlay is present:

```powershell
cargo run -- addon index search --file .\catalog\community-addon-index.toml --query "Big Wigs"
```

## Provider Quota Strategy

The shared catalog should answer normal discovery queries without calling upstream provider search
APIs. GitHub and Wago entries are curated because those hosts do not expose a robust addon-manager
search catalog. Install and update still resolve the current artifact from the package's provider
identity when the user chooses an entry.

GitHub resolution works anonymously, but anonymous API quota is small and shared by IP. Users who
install or validate many GitHub-backed packages should set `HEARTHSYNC_GITHUB_TOKEN` or
`GITHUB_TOKEN`; the provider and validation scripts use the same precedence.

## Growth Strategy

If the catalog grows beyond a small reviewable set, split the governance overlay by provider family
or host under `catalog/` and keep a top-level manifest. Do not create a separate repository yet
unless community maintenance volume makes the extra sync layer worth it.

A separate repo becomes useful only when:

- the catalog has many entries and independent contributors
- the metadata needs a release cadence separate from the code
- multiple clients want to consume the same canonical source map

Until then, keeping the catalog in-tree is simpler, easier to validate, and less brittle for an
early-stage tool.
