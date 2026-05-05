# WoW Addon Provider Ecosystem TODO

## Current Focus

P6 Wago provider implementation is in progress. Wago is being added as the first new real provider
using a typed source variant, while WoWInterface remains deferred until there is a documented public
metadata/download API or explicit third-party manager permission.

## Refactor Rules

- Preserve the archive-to-prepared-package boundary.
- Preserve existing source serialization while the registry refactor lands.
- Fail unsupported provider capabilities before expensive work when possible.
- Keep source attribution intact.
- Keep provider-specific policy semantics inside provider modules.
- Do not introduce a generic live-directory source that points back at the mutable WoW install.

## P0 - Baseline Review

- [x] Review current source/provider/download/update boundaries.
- [x] Confirm install/update execution after package preparation is not the main refactor target.
- [x] Record this provider workstream as separate from general bundle/config core safety work.

Exit criteria:

- The team agrees that the first provider slice targets source parsing, artifact resolution, cache,
  search, and dependency capability rather than addon mutation execution.

## P1 - Provider Registry Skeleton

Goal: route current provider families through one registry-oriented interface while preserving
existing behavior.

- [x] Define provider ids and source family ids.
- [x] Add a registry object that owns source parser and materializer dispatch.
- [x] Move current local archive, HTTP archive, GitHub, and CurseForge source dispatch behind
  registry adapters.
- [x] Keep existing `AddonSourceRef` serialization compatible.
- [x] Keep current install/update/index/lock tests passing without behavioral changes.

Exit criteria:

- Adding a new provider does not require editing install/update execution modules.
- Current source kinds continue to parse, serialize, materialize, and update as before.

## P1.5 - Descriptor-Backed Registry

Goal: make registry entries self-describing before adding a new real provider.

- [x] Add provider descriptors separate from app-facing capability projection.
- [x] Route source parsing/materialization through registered built-in provider adapters.
- [x] Make source family ids string-backed so future provider family ids can cross the app boundary
  without another DTO enum expansion.
- [x] Derive source capabilities from descriptors rather than maintaining a separate hard-coded
  capability table.
- [x] Preserve current local archive, HTTP archive, GitHub Releases, and CurseForge behavior.

Exit criteria:

- Runtime capability output is descriptor-derived.
- Provider descriptor ids and source family ids are stable, non-empty, and tested.
- Future provider work can start from a new adapter descriptor without changing install/update
  execution modules.

## P2 - Capability-Owned Policy

Goal: move provider-specific update policy semantics out of broad generic matches.

- [x] Model provider support for release channel and prerelease selection.
- [x] Model provider support for exact version/file/artifact pinning.
- [x] Move CurseForge file-id pin handling into the CurseForge adapter.
- [x] Move GitHub tag pin handling into the GitHub adapter.
- [x] Return structured unsupported-policy errors before package preparation.

Exit criteria:

- Provider-specific pin logic lives with the provider that understands the remote artifact model.
- App results can explain which policy capabilities a source supports.

## P3 - Catalog Aggregation

Goal: make addon search a multi-provider catalog query instead of a CurseForge-only adapter.

- [x] Define catalog provider capability.
- [x] Support provider-scoped search requests.
- [x] Support aggregate search across configured catalog providers.
- [x] Preserve current CurseForge search behavior.
- [x] Project partial provider failures into app-facing results without hiding successful providers.

Exit criteria:

- Search can return results from more than one provider family.
- Future GUI screens can show provider-specific availability and failure details.

## P4 - Dependency Capability

Goal: keep dependency installation provider-owned and explicit.

- [x] Move current CurseForge dependency strategy behind a provider adapter.
- [x] Keep unsupported dependency capability as a first-class result.
- [x] Add preflight checks for index/lock/update flows through the registry.
- [x] Keep missing-required-only semantics explicit.

Exit criteria:

- Dependency behavior is advertised by source provider, not inferred from generic source enum
  matching scattered through update/index/lock paths.

## P5 - Source Schema Evolution Decision

Goal: decide whether the existing closed source enum remains good enough after the registry exists.

- [x] Inventory the edits required to add Wago through the registry.
- [x] Inventory the edits required to add WoWInterface through the registry.
- [x] Decide whether to keep explicit typed enum variants or add a schema-v2 provider payload.
- [x] Record the migration triggers and legacy-read requirements to use if schema-v2 becomes
  necessary later.

Exit criteria:

- Source persistence evolves deliberately rather than as a side effect of the first new provider.
- Decision: keep explicit typed source variants for the next real provider; defer schema-v2 until a
  provider or custom catalog requires dynamic identity payloads. See `source-schema-decision.md`.

## P6 - New Provider Slice

Goal: add one new real provider only after the registry and capability seams are in place.

- [x] Research Wago source identity, official endpoints, artifact download rules, and terms.
- [x] Research WoWInterface source identity, official endpoints, artifact download rules, and terms.
- [x] Pick one provider based on stable metadata availability and attribution safety.
- [x] Add Wago provider contract tests beside the provider module.
- [x] Add app-level install/projection tests; keep search disabled until Wago exposes a structured
  catalog payload.

Exit criteria:

- The first new provider lands without expanding central dispatch modules in a way that recreates
  the old coupling.
- Decision: implement Wago first with `project_id` plus optional `release_id` as the typed source
  identity. See `provider-research.md`.
