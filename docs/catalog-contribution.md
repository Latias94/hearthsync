# Catalog Contribution Workflow

Date: 2026-05-06

Status: technical-preview workflow for the in-repository community catalog.

The community catalog is a metadata-only source map. It should make common addons discoverable
without turning HearthSync into a mirror, search scraper, or redistribution service.

## Contribution Rules

- Add source identities, not downloaded archives.
- Prefer upstream-owned provider references over mirrors.
- Do not add hidden download URLs when a first-class provider source exists.
- Do not pin transient release tags in the community catalog unless the provider source cannot be
  resolved safely without one.
- Keep exact reproducible versions in addon locks, private indexes, or user manifests.
- Keep the TOML index and governance JSON in the same pull request.
- Validate with read-only commands before proposing the change.

## Add One Package

1. Choose a stable package id.

   Use lowercase ASCII with `-` only when needed, for example `weakauras`, `bigwigs`, or
   `details`.

2. Add a TOML package to `catalog/community-addon-index.toml`.

   Required fields:

   - `id`
   - `name`
   - `version`
   - `source`
   - `source_url`
   - `website_url`
   - `addon_directories`
   - `supported_flavors`

   `version` is the last verified upstream snapshot for review context. It is not the install-time
   replay source for floating provider references.

3. Add identity hints.

   `match_package_ids` should include known package ids HearthSync may have generated earlier from
   provider identities. `addon_directories` should list every top-level addon folder expected after
   extraction.

4. Add governance metadata to `catalog/community-addon-index.governance.json`.

   Add one entry with matching `id`, aliases, upstream hosts, source attribution, maintainer,
   status, confidence, `last_verified_at`, and notes.

5. Run local schema and search checks:

   ```powershell
   cargo run -- addon index inspect --file .\catalog\community-addon-index.toml
   cargo run -- addon index validate --file .\catalog\community-addon-index.toml
   cargo run -- addon index search --file .\catalog\community-addon-index.toml --query "<name-or-alias>"
   ```

6. Run the read-only validation script:

   ```powershell
   .\scripts\catalog-readonly-validation.ps1
   ```

   The script validates schema health, governance coverage, alias search, and live provider dry-run
   probes against a synthetic install. It does not write addon files into a real WoW installation.

## Source Selection

Prefer these source forms:

- GitHub Releases: `source = { kind = "github_release", owner = "...", repo = "..." }`
- Wago: `source = { kind = "wago_addon", project_id = "..." }`
- Tukui: `source = { kind = "tukui_addon", slug = "..." }`
- CurseForge: `source = { kind = "curseforge_mod", mod_id = 12345 }`
- Direct HTTP archive only when no structured provider source exists.

Provider-specific notes:

- GitHub should point to the owning repository. Use a tag or asset name only when a floating release
  cannot be selected safely.
- Wago should point to the stable project id. Release ids belong in lockfiles unless a specific
  replay is required.
- Tukui exposes current-version metadata. Treat the optional version as a current-version guard, not
  a historical release replay.
- CurseForge validation requires `HEARTHSYNC_CURSEFORGE_API_KEY`; keep the source as provider API
  metadata, not a copied file URL.
- HTTP archives must be direct zip URLs with clear upstream attribution.

## Updating Existing Rows

Use updates when the upstream source identity, addon directories, supported flavors, or governance
confidence changes. Do not churn `version` or `last_verified_at` unless a real verification happened.

When a source identity changes, keep older `match_package_ids` if they help existing tracked users
attach or relink safely.

## Local Registry Assisted Curation

When you already track an addon locally, these commands can help derive hints without guessing:

```powershell
cargo run -- addon index scaffold --install $Install --flavor $Flavor --file .\target\research\local-index.toml --index-name "Local Draft"
cargo run -- addon index suggest --install $Install --flavor $Flavor --file .\catalog\community-addon-index.toml --name WeakAuras
```

Review generated output before copying it into the community catalog. Local registry state may
contain personal paths, local archive sources, or temporary package ids that are not appropriate for
shared metadata.

## Review Checklist

- [ ] The package source points at the original upstream project.
- [ ] No addon archive or extracted payload was committed.
- [ ] `addon_directories` matches the real extracted top-level folders.
- [ ] `supported_flavors` reflects verified provider metadata or extracted `.toc` files.
- [ ] Governance aliases cover common user search terms.
- [ ] `source_attribution` is clear enough for review.
- [ ] `cargo run -- addon index validate` passes.
- [ ] `scripts/catalog-readonly-validation.ps1` passes, or the skipped provider rows are explained.
