# HearthSync Catalog

This directory holds the repository-owned addon catalog.

It is metadata-only:

- no addon zip archives
- no user registry state
- no backup or restore payloads
- no hidden mirror of provider archives

## Current Shape

The first catalog file is [`community-addon-index.toml`](./community-addon-index.toml). It reuses
the existing addon-index schema so the catalog can be validated by the current `addon index
inspect` flow. That keeps the catalog reviewable without adding a second parser before the shape
has proven itself.

In practice, each package record is a curated source mapping:

- `id` is the stable package key inside HearthSync.
- `name` is the human-facing addon name.
- `version` is the current upstream version snapshot, not a promise of historical replay.
- `source` is the canonical provider identity.
- `source_url` is the best machine-followable upstream location for the source.
- `website_url` is the human landing page.
- `match_package_ids` preserve historical package-id continuity across source drift.
- `addon_directories` enumerate the exact folders the archive should expand into.
- `supported_flavors` list the WoW flavors the package is known to support.

## Source Policy

- Prefer upstream-owned source references.
- Keep GitHub entries pointed at the release or repository that owns the archive, not a mirror.
- Keep Wago entries pointed at the project id and optional release id only.
- Keep Tukui entries pointed at the official slug and optional current-version guard only.
- Do not store downloaded archives, extracted payloads, or user registry state.
- Do not store anything that would make the catalog a hidden redistribution layer.

## Why Reuse The Addon Index Schema For Now

- The current CLI already validates this shape.
- Existing exact-hint validation tells us whether a catalog entry can be matched safely.
- The catalog can seed the product without locking us into a second schema too early.
- The provider registry already knows how to consume the same source references.

This is a bootstrap shape, not a final promise. If the catalog later needs fields that the
addon-index schema cannot represent cleanly, introduce a catalog v2 rather than bending install-time
index semantics indefinitely.

## What A Future Catalog V2 Would Add

Only add these when there is a consumer and a migration plan:

- `aliases`: search aliases and historical product names
- `upstream_hosts`: canonical host labels such as `github`, `wago`, or `tukui`
- `last_verified_at`: when the mapping was last read-only validated
- `status`: active, legacy, archived, or blocked
- `confidence`: how strong the mapping is
- `notes`: maintainer-facing editorial notes
- `maintainer`: who owns the mapping

If the catalog needs richer editorial metadata, keep it outside the addon-install schema so the
install/update path stays conservative.

## Validation

The catalog file can be validated with the existing addon-index inspection command:

```powershell
cargo run -- addon index inspect --file .\catalog\community-addon-index.toml
```

That checks schema validity, source reference validity, and addon-directory portability.

For repeatable local validation, use [`scripts/catalog-readonly-validation.ps1`](../scripts/catalog-readonly-validation.ps1).
It verifies schema health and search wiring without mutating addon state.

Future provider probes should verify source refs against live upstream endpoints without
downloading archives. Those probes should fail fast on stale mappings and still avoid local addon
state changes.

The catalog is also searchable through the local addon-index search entry:

```powershell
cargo run -- addon index search --file .\catalog\community-addon-index.toml --query ElvUI
```

## Growth Strategy

If the catalog grows beyond a small reviewable set, split it by provider family or host under
`catalog/` and keep a top-level manifest. Do not create a separate repository yet unless community
maintenance volume makes the extra sync layer worth it.

A separate repo becomes useful only when:

- the catalog has many entries and independent contributors
- the metadata needs a release cadence separate from the code
- multiple clients want to consume the same canonical source map

Until then, keeping the catalog in-tree is simpler, easier to validate, and less brittle for an
early-stage tool.
