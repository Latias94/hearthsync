# Addon Source Schema Decision

Status: accepted on 2026-05-05.

## Decision

Keep `AddonSourceRef` as an explicit typed enum for the next real provider slice. Do not introduce a
schema-v2 dynamic provider payload before adding Wago or WoWInterface.

This is a deliberate deferral, not a rejection of schema-v2. The current registry refactor moved
provider behavior behind descriptors and adapters, but source identity is still part of stable user
artifacts:

- managed addon registry
- addon lock files
- curated addon indexes
- cache metadata
- app response DTOs
- package id derivation
- addon-index matching strategies
- lock planning and validation

A dynamic payload would require a persistence migration, app DTO expansion, cache compatibility
rules, and matching semantics before it can prove value. Adding one typed provider variant keeps the
first new-provider slice reviewable and preserves existing files.

## Current Typed Source Touch Points

Adding a new typed source family currently requires edits in these areas:

- `AddonSourceRef` serialization, display, and validation in `provider/source.rs`.
- `AddonSourceFamily::from_source` and the built-in adapter list in `provider/registry.rs`.
- Provider parser/materializer/search/policy/dependency/cache behavior in or under
  `provider/<provider>`.
- App-facing `AddonSourceResult` and `AddonSourceKindResult` projection.
- Package id derivation in `package_prep/package_id.rs`.
- Addon-index source identity and source-family matching in `index/matching/strategies.rs`.
- Dependency identity/satisfaction only if the provider supports dependencies.
- Cache namespace and remote repair only if the provider supports remote validators.
- Lock/index validation tests for serialized source contracts.

These are central edits, but they are now mostly declarative or provider-owned. They are lower risk
than a schema-v2 migration because all current persisted shapes remain type-checked and testable.

## Wago Inventory

A typed Wago source can be added without schema-v2 if P6 research confirms stable source identity and
download metadata.

Expected edit set:

- Add `AddonSourceRef::WagoAddon` with provider-owned fields chosen from P6 research.
- Add `AddonSourceFamily::WAGO_ADDON` and a `BuiltinAddonProviderAdapter::Wago`.
- Add `provider/wago` for input parsing, remote DTO validation, artifact resolution, catalog result
  projection, and optional cache validators.
- Add app source projection fields only for stable identifiers that the UI must display directly.
- Add package id and addon-index matching rules based on stable Wago identity.
- Add cache repair only if Wago exposes reliable remote validators or immutable artifact identity.
- Add provider contract tests before enabling install/update/search paths.

No schema-v2 is needed for this if Wago identity is compact and stable, for example one project id
plus an optional exact artifact/version pin.

## WoWInterface Inventory

A typed WoWInterface source can also be added without schema-v2 if P6 research confirms stable
metadata and a safe download rule.

Expected edit set:

- Add `AddonSourceRef::WowInterfaceAddon` with provider-owned fields chosen from P6 research.
- Add `AddonSourceFamily::WOWINTERFACE_ADDON` and a `BuiltinAddonProviderAdapter::WoWInterface`.
- Add `provider/wowinterface` for parsing, remote DTO validation, artifact resolution, catalog
  projection, and optional cache validators.
- Add app source projection fields only for stable identifiers that the UI must display directly.
- Add package id and addon-index matching rules based on stable WoWInterface identity.
- Add cache repair only when the provider exposes enough metadata to avoid fragile scraping.
- Add provider contract tests before enabling install/update/search paths.

No schema-v2 is needed for this if WoWInterface identity is compact and stable, for example one file
or project id plus an optional exact artifact pin.

## Tukui Inventory

Tukui fits the typed-source path:

- `AddonSourceRef::TukuiAddon` stores `slug` and optional `version`.
- The slug is the stable source identity.
- The version is a current-version guard and cache identity, not a historical artifact pin.
- Catalog search is provider-owned because `/addons` is a small structured Tukui payload.
- No schema-v2 is needed because the identity is compact and the provider does not require
  user-defined fields.

## When To Reopen Schema-v2

Reopen this decision if one of these becomes true:

- A provider requires unbounded or provider-defined identity fields that do not fit a small typed
  Rust variant.
- Custom local or remote catalog providers need user-defined source families.
- Two or more new providers duplicate the same generic provider/id/version/artifact structure.
- App or GUI consumers need source fields as an extensible map instead of stable typed fields.
- Cache repair or addon-index matching needs provider-specific identity comparison rules that should
  be data-driven.

If reopened, schema-v2 should be additive:

- Keep reading all current source variants.
- Add a new `provider_payload` source kind with stable `provider_id`, `source_family`, and a
  structured payload.
- Add migration tests for registry, lock, index, cache metadata, and app DTO compatibility.
- Keep typed variants for current providers until the dynamic shape proves simpler in practice.
