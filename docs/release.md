# Release Process

Date: 2026-05-06

Status: technical-preview release process. Consumer-facing binary distribution is still a release
gate, not a completed promise.

## CI Gate

The repository CI runs on Windows and macOS:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run --no-fail-fast
```

`CARGO_BUILD_JOBS=1` is used for clippy and nextest in CI to reduce memory pressure on hosted
runners. Provider live validation scripts are not part of the default PR gate because they can need
network access, credentials, provider quota, and proxy configuration.

## Local Release Checklist

Run these checks on the release machine before publishing artifacts:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo nextest run --no-fail-fast
cargo build --release --locked
```

Then run read-only provider/catalog probes when credentials and network access are available:

```powershell
.\scripts\catalog-readonly-validation.ps1
.\scripts\addon-provider-compatibility-readonly.ps1
.\scripts\config-package-compatibility-readonly.ps1
```

Use `HTTPS_PROXY`, `HEARTHSYNC_GITHUB_TOKEN`, and `HEARTHSYNC_CURSEFORGE_API_KEY` as needed. Do not
paste secret values into release notes, logs, or issue comments.

## Artifact Naming

Use explicit OS and architecture names:

- `hearthsync-<version>-windows-x86_64.zip`
- `hearthsync-<version>-macos-aarch64.tar.gz`
- `hearthsync-<version>-macos-x86_64.tar.gz`

Each archive should contain:

- the `hearthsync` binary;
- `README.md`;
- `LICENSE-APACHE` and `LICENSE-MIT` when license files are present;
- a short `checksums.txt` or adjacent `.sha256` file in the release assets.

## Checksums

Windows:

```powershell
Get-FileHash .\dist\hearthsync-<version>-windows-x86_64.zip -Algorithm SHA256
```

macOS:

```bash
shasum -a 256 dist/hearthsync-<version>-macos-aarch64.tar.gz
shasum -a 256 dist/hearthsync-<version>-macos-x86_64.tar.gz
```

Publish checksum values next to the release artifacts so users can verify downloads before running
the binary.

## Release Notes

Release notes should include:

- current product grade, for example `CLI technical preview`;
- supported operating systems and WoW flavors tested for the release;
- known provider credential requirements and quota caveats;
- config package compatibility evidence used for the release;
- upgrade or migration notes for persisted state;
- the exact git commit or tag used to build the binaries.

Do not call a release consumer-facing beta until Windows and macOS binaries with checksums are
actually published.

## Artifact Workflow

The manual `Release Artifacts` workflow builds native release binaries on Windows and macOS,
packages README and license files, generates SHA256 files, and uploads the archives as workflow
artifacts.

It intentionally does not create or publish a GitHub Release. Keep the final publish step manual
until the project has signing, changelog review, and release-note approval rules.
