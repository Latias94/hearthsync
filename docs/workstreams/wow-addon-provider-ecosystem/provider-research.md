# New Provider Research

Status: accepted on 2026-05-05; extended with Tukui and catalog research on 2026-05-06.

## Decision

Implement Wago as the first new real addon provider. Defer WoWInterface until there is a documented
public metadata/download API or explicit permission for third-party addon manager access.

Recommended Wago source shape:

```text
WagoAddon {
  project_id: String,
  release_id: Option<String>,
}
```

The Wago project id should be the persisted identity. A release id is an optional exact artifact pin.
Do not persist slugs or signed download URLs as canonical identity.

## Wago Findings

Official documentation is focused on developer upload automation rather than client-side addon
installation. It documents API-key authentication, the `X-Wago-ID` TOC marker, release stability
values, game patch metadata, and upload through `POST /api/projects/<project id>/version`.

Useful official and observed endpoints:

- `GET /api/data/game` returns stability values, patch lists, TOC suffixes, and live patch names.
- `GET /addons/{project_id}` loads the addon details page and exposes Inertia props including
  `addon.id`, `display_name`, `slug`, website/source/support URLs, categories, and verification
  flags.
- `GET /addons/{project_id}/versions?stability=stable&page=1` redirects to the slug URL but still
  returns release props. Each release includes `id`, numeric `addon_id`, `size`, `label`,
  `stability`, timestamps, supported patch arrays, and a signed `download_link`.
- `GET /?search=<query>` returns a search page, but the addon list entries are rendered HTML strings
  rather than structured addon DTOs.
- Signed `download_link` values redirect to `cdn.wago.io/public/addons/{project_id}/...zip`.
- Direct `GET /download/{release_id}` returned 403 during research. `GET /external/download/{id}`
  returned 401. The provider should obtain a fresh signed link from the versions page when
  materializing an artifact.

Implementation implications:

- Persist `project_id` and optional `release_id` only.
- Resolve the latest release from the versions page using release stability and target flavor
  filters.
- Treat signed URLs as per-operation artifacts and cache only the downloaded archive plus validated
  metadata, not the signed URL itself.
- Add a provider-owned Inertia parser with fixture tests; do not let HTML/JSON parsing leak into
  shared registry code.
- Mark dependency resolution unsupported unless Wago exposes dependency metadata later.
- Search can be added after install/update because the current search payload is less structured
  than details/releases. If added, isolate HTML extraction and keep failure handling provider-owned.
- Use a project-specific user agent and conservative request throttling. The current public docs do
  not describe a client download API contract.

## WoWInterface Findings

WoWInterface has stable-looking numeric addon ids and TOC metadata such as `X-WoWI-ID`. Public addon
URLs also include numeric ids, for example `downloads/info25043-...html`.

The documented API is not a public installer API:

- `GET /addons/list.json` and `GET /addons/details/<id>.json` require `x-api-token` and return
  addons the authenticated account can access.
- `POST /addons/update` is an author/team upload endpoint.
- Forum discussion says WowUp used an undocumented Minion API, so that path is not a stable public
  contract.

Download and access concerns:

- Direct shell access to `www.wowinterface.com` and `mmoui.com` hit Cloudflare challenge pages during
  research, which makes unattended CLI access unreliable.
- CDN landing pages can expose temporary file URLs, but that is a browser download path, not a
  documented third-party manager API.
- MMOUI terms require use through the provided interface and prohibit automated access through
  scripts or web crawlers without a separate agreement.

Implementation implication:

- Do not implement WoWInterface by scraping pages or using the undocumented Minion API.
- Existing direct HTTP archive support is enough for users who manually provide a WoWInterface zip
  URL.
- Reopen WoWInterface only if a documented public metadata/download API is available or permission is
  granted.

## Tukui Findings

Tukui is a good narrow provider candidate because the current public API exposes the exact two
high-value UI packages that still matter for author config-package workflows:

- `GET https://api.tukui.org/v1/addon/elvui`
- `GET https://api.tukui.org/v1/addon/tukui`
- `GET https://api.tukui.org/v1/addons`

The single-addon response includes stable fields that are enough for install/update without scraping:

- `slug`
- `name`
- `url`
- `version`
- `patch`
- `web_url`
- `git_url`
- `small_desc`
- `directories`

Implementation implications:

- Use `TukuiAddon { slug, version }` as a typed source ref.
- Parse `tukui:<slug>[@current-version]` for CLI/source input.
- Treat `version` as a current-version guard and cache identity, not as a promise that historical
  Tukui releases can be replayed later.
- Keep addon policy version pin unsupported unless Tukui later exposes a historical release API.
- Enable Tukui catalog search because `/addons` exposes a small structured catalog.
- Keep dependency resolution and remote cache repair unsupported; the API does not expose
  dependency or strong archive validator contracts.

## External Manager Source Management Notes

Other open-source managers reinforce two different patterns:

- Strongbox searches a downloaded catalogue rather than asking every provider live for every query.
  It also maintains a separate public `strongbox-catalogue` repository with per-host JSON catalogues
  such as `github-catalogue.json`, `full-catalogue.json`, and `short-catalogue.json`.
- instawow supports multiple providers directly and uses a collated add-on catalogue updated once
  daily for fuzzy search and download scoring. It accepts source-specific URIs and provider URLs.
- WowUp distinguishes provider-backed searchable sources from URL imports. Its guide says Get
  Addons lists/searches providers that expose catalogs, while GitHub and generic zip URLs can still
  be installed through URL import.

HearthSync should follow a hybrid version of those patterns:

- Built-in providers remain responsible for artifact resolution and downloads from original hosts.
- The repository should keep an in-tree `catalog/` metadata layer that stores source metadata only,
  not addon archives.
- The catalog should be optional input to addon index/search/adoption workflows, not the primary
  installed-state database.
- GitHub source discovery is the first strong reason for a catalog because GitHub Releases are
  installable but not centrally searchable by addon identity.

Recommended first external catalog schema:

```toml
schema_version = 1
name = "HearthSync Community Addon Catalog"
updated_at = "2026-05-06T00:00:00Z"

[[packages]]
id = "weakauras"
name = "WeakAuras"
source = { kind = "github_release", owner = "WeakAuras", repo = "WeakAuras2", asset_name = "WeakAuras.zip" }
website_url = "https://github.com/WeakAuras/WeakAuras2"
addon_directories = ["WeakAuras", "WeakAurasOptions"]
supported_flavors = ["retail"]
aliases = ["wa", "weak auras"]
upstream_hosts = ["github"]
```

Repository policy:

- Do not store downloaded addon zips.
- Require upstream source URL, source kind, addon directories, supported flavors, and attribution.
- Prefer generated validation reports over hand-curated trust.
- Accept community PRs for GitHub/Wago/Tukui source mappings.
- Keep CurseForge entries optional because official API usage requires caller credentials and
  author/platform policy may affect availability.

## Sources

- Wago API docs: https://docs.wago.io/
- Wago addon site and routes observed on 2026-05-05: https://addons.wago.io/
- Wago Terms of Service route: https://addons.wago.io/agreements/terms-of-service
- WoWInterface update API forum thread: https://www.wowinterface.com/forums/showthread.php?t=51835
- WoWInterface WowUp/Minion discussion: https://wowinterface.com/forums/showthread.php?t=59124
- MMOUI Terms of Service: https://mmoui.com/?tos=
- Tukui addon API: https://api.tukui.org/v1/addon/elvui
- Tukui catalog API: https://api.tukui.org/v1/addons
- Strongbox README and catalogue behavior: https://github.com/ogri-la/strongbox
- Strongbox public catalogue repository: https://github.com/ogri-la/strongbox-catalogue
- instawow README: https://github.com/layday/instawow
- WowUp Get Addons guide: https://wowup.io/guide/get-addons/overview
