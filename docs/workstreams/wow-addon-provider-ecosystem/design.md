# WoW Addon Provider Ecosystem Design

## Status

Active on 2026-05-04.

This workstream is a focused refactor track for HearthSync's multi-source addon acquisition
architecture. It exists because provider growth is now a core product goal and the current
`AddonSourceRef` plus `DefaultAddonProvider` shape will become expensive if every new provider
requires edits across the whole addon stack.

The core sync workstream remains responsible for bundle/config/app safety semantics. This workstream
owns the source-provider front half:

- source identity parsing and persistence
- catalog search
- version and release policy
- artifact resolution
- download/cache validators
- dependency capability
- provider-specific metadata projection

## Problem Statement

HearthSync currently supports:

- local zip archives
- direct `http(s)` zip URLs
- GitHub Releases
- CurseForge project/file references

The current design is adequate for these sources, but it is not yet ideal for a broader open-source
provider ecosystem including Wago, WoWInterface, custom catalogs, and additional direct source
families.

The pressure point is not addon installation after an archive is prepared. The pressure point is the
path from user source input to a validated local archive:

```text
source input / persisted source
  -> parse
  -> resolve provider artifact
  -> apply version policy
  -> download or reuse cache
  -> resolve dependencies
  -> materialized addon archive
```

Today that path is coordinated through a closed `AddonSourceRef` enum and provider-wide `match`
statements. New provider families therefore create cross-cutting edits.

## Current Shape

Healthy boundaries to preserve:

- `package_prep`: archive to prepared addon package
- addon mutation execution: prepared package to installed AddOns plus registry updates
- addon lock planning and apply
- addon index install/update orchestration
- app task progress and cancellation
- HTTP transport injection for tests
- cache metadata validation and repair logic

Boundaries under pressure:

- `AddonSourceRef` is a closed enum used by registry, index, lock, policy, cache, and app DTOs.
- `AddonProvider` mixes source materialization, source input parsing, search, dependency resolution,
  and cache maintenance.
- `source_adapter` is currently mostly a CurseForge adapter rather than a generic provider registry.
- `materialize` is the central source dispatch module and will keep growing with every new source.
- provider pin/release policy is not capability-owned enough.
- `search_addons` is not a multi-provider catalog aggregator.

## Target Shape

The target architecture is a provider registry with small capability traits behind the app/domain
boundary:

```text
AddonService / AddonIndex / AddonLock
  -> AddonSourceRegistry
    -> SourceParser
    -> ArtifactResolver
    -> CatalogSearchProvider
    -> DependencyResolver
    -> SourcePolicyAdapter
    -> CacheValidatorAdapter
  -> Archive materialization cache
  -> package_prep
  -> mutation / lock / bundle flows
```

Provider implementations own provider-specific semantics:

- URL syntax and canonical source identity
- remote DTO validation
- release channel and prerelease policy
- version pin and file pin semantics
- artifact filename and download URL validation
- cache key and remote validator projection
- dependency support level
- search result projection

The shared addon domain consumes provider-neutral outputs:

- canonical source reference
- resolved artifact metadata
- local archive path
- dependency source references
- display metadata for registry, lock, and app results
- explicit capability results

## Source Identity Strategy

The first refactor should avoid a breaking persistence rewrite unless it is clearly necessary.

Recommended approach:

1. Keep the existing serialized source kinds working.
2. Route all parsing/materialization through a registry interface.
3. Move provider-specific `match` logic behind provider adapters.
4. Add new providers through the registry path.
5. Only introduce a schema-v2 dynamic source representation after the provider registry proves the
   shape.

This keeps existing `addons.toml`, `lock.toml`, and addon index files usable during the refactor.

P5 decision: keep explicit typed `AddonSourceRef` variants for the next real provider. The registry
is now descriptor-backed enough that adding one provider through a typed variant is cheaper and
safer than migrating registry, lock, index, cache metadata, app DTOs, package id derivation, and
addon-index matching to a dynamic payload. Schema-v2 should be reopened only if Wago,
WoWInterface, or custom catalogs prove that provider identity needs unbounded provider-defined
fields. The detailed inventory and reopen criteria live in `source-schema-decision.md`.

The second registry slice keeps that persistence decision intact but changes capability discovery
to be descriptor-driven:

- `AddonProviderDescriptor` is the provider-facing shape for source family id, provider id,
  provider name, input prefix, supported operations, and policy support.
- `AddonProviderSourceCapability` remains the app-facing flattened projection used by CLI and
  future GUI surfaces.
- `AddonSourceFamily` is a stable string-backed id value rather than an app-facing closed enum, so
  future source family ids such as Wago, WoWInterface, or custom catalogs can cross the runtime
  capability boundary before a schema-v2 source payload is chosen.
- Built-in providers are registered as adapter entries. The adapter list currently contains local
  archive, HTTP archive, CurseForge, GitHub Releases, and Wago.

Provider-owned policy is layered on top of those descriptors:

- addon policy storage still records generic pins and release/prerelease preferences
- `AddonProvider::apply_source_policy` resolves those generic preferences against provider
  capabilities before package preparation
- CurseForge owns file-id pin application
- GitHub Releases owns version/tag pin application
- unsupported policy errors include provider id, source family id, source display name, and the
  unsupported capability

## Provider Capability Model

Each provider should advertise capabilities explicitly:

- can parse CLI source input
- can materialize a persisted source reference
- can search a catalog
- can resolve latest artifact
- can apply stable/beta/alpha policy
- can apply provider-specific pin policy
- can resolve dependencies
- can provide strong remote validators
- can repair cached artifacts

Unsupported capabilities should fail before expensive work when possible. The app boundary should
be able to project these capabilities for future GUI screens.

## Catalog Search Model

Search should become aggregation over one or more catalog providers:

- CurseForge search
- future Wago search
- future WoWInterface search
- optional custom local index search

Search results should include:

- provider id
- provider project id
- optional provider file/artifact id
- source install hint
- website URL
- source kind/capability labels
- target flavor compatibility when known

Search aggregation must keep provider failures structured. One provider being unavailable should
not necessarily hide results from other providers unless the caller requested that provider
explicitly.

The P3 implementation keeps the current CurseForge result projection but changes the search
contract:

- `AddonSearchRequest` carries an optional provider id.
- `AddonProvider::search_addon_catalog` returns successful results plus provider-level failures.
- `DefaultAddonProvider` routes catalog search through descriptor-backed registry adapters whose
  `can_search` operation is the catalog provider capability.
- Aggregate search records partial provider failures while preserving the existing all-failed
  CurseForge error behavior.
- App and CLI results expose `provider_id`, `failure_count`, and provider failure details; the CLI
  supports `addon search --provider <id>`.

## Dependency Capability Model

Dependency installation is provider-owned rather than inferred from generic source matching:

- `AddonProviderDescriptor.operations.dependency_resolution` advertises support and strategy.
- CurseForge owns required-dependency extraction from file metadata.
- The currently supported strategy remains explicitly `MissingRequiredOnly`.
- Unsupported dependency resolution is still projected as a first-class app capability.
- Addon update and addon-index update preflight dependency support before package preparation.
- Addon lock apply does not have dependency policy semantics; it applies the exact lock package
  source set and still routes source materialization through the provider registry.

## Non-Goals

- Rewriting addon file mutation and rollback.
- Replacing bundle or config package planning.
- Full async conversion.
- Adding a plugin runtime or dynamic third-party provider loading.
- Making registry state the primary sharing format.

## First Providers After Refactor

The provider registry should first preserve current behavior:

- local archive
- HTTP archive
- GitHub Releases
- CurseForge
- Wago
- Tukui

Remaining provider-family candidates:

1. WoWInterface, if source identity and artifact URLs can be resolved without scraping fragile
   pages.
2. Custom manifest/index-backed catalog search for user-maintained source lists.

P6 research selects Wago as the first new real provider. The source identity should be a typed
`WagoAddon` reference with `project_id` and an optional exact `release_id` pin. Wago exposes enough
project, release, patch, and signed download metadata through its addon site to support an isolated
provider implementation, but the implementation should treat these as provider-owned web contracts
rather than shared core assumptions. WoWInterface is deferred because its documented API is
author/update oriented, public page access is Cloudflare-gated, and the available download flow is
not a documented third-party manager contract. The detailed research is in `provider-research.md`.
The implementation keeps Wago search disabled for the first slice because the available search page
is less structured than the release metadata needed for install/update.

P7 implementation adds Tukui as a narrow official-API provider with `TukuiAddon { slug, version }`.
The `version` field is treated as current-version guard and cache identity, not historical release
replay. Tukui catalog search is enabled because `/addons` is structured and small; dependency
resolution, release-channel policy, and cache remote validation remain unsupported until Tukui
exposes those contracts.

Provider additions should be blocked on source attribution and hosting-term respect. HearthSync
should continue to download from original provider sources or user-provided source URLs rather than
becoming a redistribution platform.

## Community Metadata Catalog

GitHub Releases, Wago project ids, and Tukui slugs are usable source identities, but only CurseForge
and Tukui currently expose straightforward search catalogs. GitHub in particular needs a discovery
layer if HearthSync wants good open-source addon search without scraping.

The recommended product direction is an in-tree `catalog/` metadata layer, starting with
`catalog/community-addon-index.toml`:

- stores source metadata only, not archives
- reuses the existing addon index package schema where possible
- validates package source refs through read-only dry-run probes
- accepts community PRs for GitHub/Wago/Tukui mappings
- can be consumed as an optional local addon index now and mirrored remotely later if needed

This follows the stronger parts of Strongbox and instawow: a public source catalogue helps discovery,
while provider adapters still download from original hosts and enforce host-specific rules. Keeping
the first catalog in-tree also makes code review, validation, and release coordination simpler than
splitting the metadata source of truth into another repository too early.

## Relationship to Sharing

For cross-machine sharing, the recommended source-of-truth should be:

- addon lockfile
- remote provider sources where possible
- sidecar `sources.toml` and `sources/*.zip` only for personal migration or license-safe packages

The app-data addon registry remains local managed state. It should not be presented as the primary
portable sharing artifact.
