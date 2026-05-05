# Config Package Compatibility Hardening

Date: 2026-05-05

Status: active hardening track after the shareable config package MVP.

## Success Criteria

This track is useful when it produces concrete compatibility evidence instead of only adding more
green tests. The minimum hardening loop is:

1. maintain a compatibility matrix for real source families and representative synthetic fixtures;
2. add sanitized Lua SavedVariables reductions when local or real package shape evidence shows a
   relevant marker/container pattern;
3. keep real-user package checks read-only by default and write only aggregate, privacy-preserving
   summaries under `target/research`;
4. keep broad desktop migration claims separate from the narrower fail-closed compatibility
   evidence.

## Read-Only Verification Flow

Use [config-package-compatibility-readonly.ps1](../../../scripts/config-package-compatibility-readonly.ps1)
from the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\config-package-compatibility-readonly.ps1 -SkipBuild -MaxModuleSamples 4
```

By default the script runs only inspection commands:

- `hearthsync --json config inspect --source <wow-retail-root>`
- `hearthsync --json external-package inspect --source <real-package>`

It does not run `apply`, does not create a bundle, and does not write to the live WoW tree or source
archives in that default mode. Its report stores labels, aggregate counts, layouts, warnings,
public-sharing summaries, and short addon/interface samples only. It intentionally does not store
normalized entry lists or file contents.

To also verify the apply planning surface without touching a live game tree, pass
`-IncludeSyntheticPlanDryRun`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\config-package-compatibility-readonly.ps1 -SkipBuild -MaxModuleSamples 4 -IncludeSyntheticPlanDryRun
```

That mode creates a minimal synthetic WoW installation under the run's `target/research/...`
directory and runs `hearthsync --json external-package apply --dry-run` for inspected external
package inputs. It records only aggregate plan/apply counts and still omits operation destinations,
normalized entry lists, and file contents.

Operators can pass additional real package paths through `-ExternalPackageSources`. Those packages
are inspected as generic external-package inputs and summarized with the same privacy boundary.

## Current Compatibility Matrix

Latest local read-only run:
`target/research/config-package-compatibility-readonly-20260505-174852/compatibility-readonly-summary.md`

Latest synthetic-target dry-run smoke:
`target/research/config-package-compatibility-readonly-20260505-195018/compatibility-readonly-summary.md`

| Source family | Probe | Evidence | Result | Notes |
| --- | --- | --- | --- | --- |
| Local full retail config tree | `config inspect` | 59,842 entries normalized, 0 warnings, public sharing `review_required` | Pass | Covers AddOns, WTF, Fonts, and Interface resources from a large local install without writing or storing file contents. |
| NewBeeBox module cache: MeetingStone | `external-package inspect` | `newbeebox_addon`, 399/399 normalized, 0 warnings | Pass | Real cached addon package with NewBeeBox module naming/layout. |
| NewBeeBox module cache: BigWigs Dragonflight | `external-package inspect` | `newbeebox_addon`, 91/91 normalized, 0 warnings | Pass | Adds a second real addon family instead of relying on one module cache example. |
| NewBeeBox module cache: HandyNotes TheWarWithin | `external-package inspect` | `newbeebox_addon`, 162/162 normalized, 0 warnings | Pass | Covers addon names with underscores and expansion-specific plugin layout. |
| NewBeeBox module cache: NorthernSkyRaidTools | `external-package inspect` | `newbeebox_addon`, 289/289 normalized, 0 warnings | Pass | Covers another real module cache family. |
| NewBeeBox module cache: font package | `external-package inspect` | `newbeebox_font`, 9/9 normalized, 0 warnings | Pass | Real cached `font-*.zip` resource package; auto-detection must take precedence over the generic module-cache addon rule. |
| NewBeeBox module cache: material package | `external-package inspect` | `newbeebox_material`, 22,872/22,872 normalized, 0 warnings | Pass | Real cached `material-*.zip` resource package covering a large Interface asset set. |
| NewBeeBox account WTF cache | `external-package inspect` | `newbeebox_wtf_account`, 89/89 normalized, 0 warnings, public sharing `review_required` | Pass | Uses synthetic source-account context; no private values are emitted. |
| NewBeeBox character WTF cache | `external-package inspect` | `newbeebox_wtf_character`, 59/59 normalized, 0 warnings, public sharing `review_required` | Pass | Uses synthetic source account/server/character context; detects one source character. |
| Large wrapped author zip fixture | `cargo nextest run external_package` | `analyze_external_package_zip_handles_large_wrapped_author_package` | Covered by test | Synthetic regression with wrapper directory, many addon roots, WTF, fonts, interface assets, and archive noise. |
| Windows-source to macOS-target author package | `cargo nextest run external_package` | `apply_external_package_applies_complex_windows_author_zip_to_macos_target` | Covered by test | Verifies platform-aware normalized apply behavior and backup path on a macOS target fixture. |
| Config facade Windows-source to macOS-target | `cargo nextest run config_cli` / app config tests | Config CLI and app acceptance tests | Covered by test | Verifies the future GUI-facing config boundary rather than only the external-package engine. |
| Synthetic target NewBeeBox dry-run smoke | `external-package apply --dry-run` | 5 external package layouts passed; addon 399 planned, font 10 planned, material 22,872 planned, account WTF 89 planned, character WTF 60 planned; all had 0 writes and 0 warnings | Pass | Uses only a temporary target under `target/research`; the synthetic target contains the bootstrap `Config.wtf` only after the run. |

## Sanitized Lua Reductions

New reductions added in this hardening slice:

- `plater_profilekeys_reduced_utf8.lua`
  - Informed by a privacy-preserving marker scan that found `Plater.lua` uses `profileKeys`,
    `profiles`, spaced identity keys, and compact identity-looking keys.
  - Verifies profile key and dot-form profile table rewrites while preserving script/cache identity
    text outside known profile containers.
- `omnicd_profilekeys_char_reduced_utf8.lua`
  - Informed by a marker scan that found `OmniCD.lua` combines `profileKeys`, `profiles`, and
    `char` tables.
  - Verifies profile rewrites while preserving `char` keys because OmniCD is not identity
    allowlisted.
- `mrt_profilekeys_reduced_utf8.lua`
  - Informed by a privacy-preserving marker scan that found `MRT.lua` uses uppercase
    `ProfileKeys`/`Profiles` plus lowercase `profiles`.
  - Verifies uppercase profile container rewrites while preserving encounter/history identity text
    outside profile containers.

These are repo-authored synthetic reductions. They preserve marker/container shape and replace all
private account, realm, character, note, and scalar payload values.

## Remaining Hardening

- Add more real author-package rows beyond NewBeeBox cache packages when the user provides public
  package zips or when locally cached public packages are available.
- Expand Lua reductions for additional high-signal families before widening identity-key
  allowlists. Candidate families from local marker scans include Baganator fail-closed compact
  profiles and any future Cell/Plater/OmniCD shape variants.
- Use the optional synthetic-target dry-run mode in local verification runs whenever package apply
  planning evidence is needed without touching a live game tree.
