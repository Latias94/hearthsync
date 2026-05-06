# Addon Provider Compatibility Matrix

Purpose: keep the open multi-source addon provider contract testable without requiring a real WoW
installation to be mutated. The matrix is intentionally split between deterministic fixtures and
read-only live probes because provider websites can change independently from HearthSync releases.

Last refreshed: 2026-05-06.

## Success Criteria

- Each built-in provider family has one representative real source for read-only validation.
- Read-only validation uses `addon install --dry-run` against a synthetic WoW installation under
  `target/research`, never against a user's live AddOns directory.
- Provider downloads may use a temporary cache under the report directory; the cache is deleted by
  default after the probe unless `-KeepDownloads` is passed.
- GitHub live validation uses `HEARTHSYNC_GITHUB_TOKEN` or `GITHUB_TOKEN` when present, and skips
  GitHub rows when the available GitHub API quota is exhausted.
- CurseForge live validation is skipped unless the caller explicitly opts in and has an API key in
  `HEARTHSYNC_CURSEFORGE_API_KEY` or `CURSEFORGE_API_KEY`.
- Deterministic unit/app/CLI tests still own boundary behavior. Live probes are compatibility
  evidence, not a replacement for regression tests.

## Read-Only Probe

Run from the repository root:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/addon-provider-compatibility-readonly.ps1
```

Optional CurseForge probe:

```powershell
$env:HEARTHSYNC_CURSEFORGE_API_KEY = '<key>'
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/addon-provider-compatibility-readonly.ps1 -IncludeCurseForge
```

Outputs:

- `target/research/addon-provider-compatibility-readonly-*/addon-provider-compatibility-readonly-summary.json`
- `target/research/addon-provider-compatibility-readonly-*/addon-provider-compatibility-readonly-summary.md`

The report stores source labels, status, aggregate package/addon/file counts, and errors only. It
does not store archive contents or normalized file lists.

Latest local read-only run:
`target/research/addon-provider-compatibility-readonly-20260506-121040/addon-provider-compatibility-readonly-summary.md`

## Matrix

| Provider family | Representative source | Real-source evidence | Deterministic coverage | Read-only expected result | Notes |
| --- | --- | --- | --- | --- | --- |
| HTTP archive | `https://github.com/BigWigsMods/BigWigs/releases/download/v414.9/BigWigs-v414.9.zip` | GitHub latest release check on 2026-05-05 returned `v414.9` and `BigWigs-v414.9.zip`. | `provider::tests::http_cache`, `provider::materialize`, shared URL validation tests. | `addon install --dry-run` downloads the archive into the temporary cache, prepares addon roots, and writes no AddOns files. | Covers direct URL, cache path derivation, no provider metadata API, and transport validator behavior when the server exposes it. |
| HTTP archive: ElvUI WotLK mirror | `https://sourceforge.net/projects/elvui.mirror/files/6.09/6.09%20source%20code.zip/download` | SourceForge mirror page on 2026-05-06 exposed the `6.09 source code.zip` download for the ElvUI WotLK release. | `provider::tests::http_cache`, `provider::materialize`, shared URL validation tests. | `addon install --dry-run` downloads the mirror archive into the temporary cache, prepares addon roots, and writes no AddOns files. | Adds a larger multi-addon UI package shape from a different host while still exercising the same direct HTTP archive path. |
| GitHub Releases | `github:BigWigsMods/BigWigs@v414.9#BigWigs-v414.9.zip` | GitHub latest release check on 2026-05-05 returned `v414.9` and `BigWigs-v414.9.zip`. | `provider::github::tests`, `provider::tests::github_cache`, `provider::parse`, registry/index/lock source validation. | `addon install --dry-run` resolves the exact release and asset through the GitHub API, downloads into the temporary cache, and writes no AddOns files. | Uses the same pinned BigWigs asset as the HTTP row so the default live probe stays small and repeatable while still covering GitHub API resolution. |
| Wago addon | `wago:qv63A7Gb@vdx1042w` | Wago versions page on 2026-05-05 identified project `qv63A7Gb` as Details! Damage Meter and release `vdx1042w` as a stable processed release with download links. | `provider::wago`, `provider::materialize`, app install projection, index/lock source validation. | `addon install --dry-run` resolves a fresh signed download link from the versions page, downloads into the temporary cache, and writes no AddOns files. | Wago signed download URLs are per-operation artifacts; cache identity uses the stable project/release source ref, not the signed URL. |
| Tukui addon | `tukui:elvui` | Tukui API check on 2026-05-06 returned `slug=elvui`, `name=ElvUI`, `version=15.13`, `directories=ElvUI/ElvUI_Libraries/ElvUI_Options`, and an official download URL. | `provider::tukui`, `provider::materialize`, source serialization/projection, registry capability tests. | `addon install --dry-run` resolves the current official ElvUI API payload, downloads the official archive into the temporary cache, and writes no AddOns files. | Tukui exposes current-version metadata, not a historical release API. The optional source version is a current-version guard and cache identity, not a replay guarantee. |
| CurseForge mod | `curseforge:238222` | Public CurseForge project identity for WeakAuras; official API validation requires a caller-provided key. | `curseforge::api::tests`, `curseforge::file_validation`, `curseforge::select`, provider dependency/update tests. | Skipped by default. With `-IncludeCurseForge` and an API key, resolves a current file for the target flavor and performs dry-run preparation. | Dependency installation remains narrow: missing required dependencies only, and only for supported CurseForge sources. |

## Prompt-To-Artifact Checklist

| Requirement | Artifact / evidence |
| --- | --- |
| Establish a real-source compatibility matrix | This document's Matrix section. |
| Provide read-only verification flow | `scripts/addon-provider-compatibility-readonly.ps1` and the Read-Only Probe section. |
| Cover GitHub | Exact-source matrix row plus `provider::github::tests::fetch_github_release_with_client_percent_encodes_tag_path_segment` and parser/source validation tests. |
| Cover Wago | Exact-source matrix row plus Wago artifact resolution and invalid download URL contract fixture. |
| Cover Tukui | Exact-source matrix row plus Tukui parser, current-version guard, catalog search, and materialization fixtures. |
| Cover CurseForge | Opt-in matrix row plus existing API/file/dependency fixtures; live probe gated by API key. |
| Cover HTTP | Direct URL matrix row plus shared HTTP URL validation and cache/materialization fixtures. |
| Add CLI regression | `cli::args::tests::addon::parses_top_level_addon_install_with_provider_compatibility_runtime_options`. |
| Add app regression | `core::app::addon::tests::install::addon_service_install_rejects_invalid_provider_source_before_mutating_installation`. |

## Remaining Hardening Ideas

- Add a CI-ignored live probe wrapper if the project later standardizes an opt-in network test
  profile.
- Add a fixture reduction whenever a live probe fails because a provider changed response shape.
- Extend the matrix if WoWInterface or custom indexes become first-class provider families.
