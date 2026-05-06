param(
    [string]$CatalogPath = ""
)

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($CatalogPath)) {
    $CatalogPath = Join-Path $RepoRoot "catalog\community-addon-index.toml"
}

Push-Location $RepoRoot
try {
    $inspectJson = cargo run --quiet -- --json addon index inspect --file $CatalogPath | Out-String
    $inspect = $inspectJson | ConvertFrom-Json

    if ($inspect.blocking_warning_count -ne 0) {
        throw "Catalog inspect reported blocking warnings."
    }

    if ($inspect.package_count -lt 1) {
        throw "Catalog inspect returned no packages."
    }

    $searchJson = cargo run --quiet -- --json addon index search --file $CatalogPath --query ElvUI --limit 5 | Out-String
    $search = $searchJson | ConvertFrom-Json

    if ($search.returned_package_count -lt 1) {
        throw "Catalog search did not return any packages for ElvUI."
    }

    if (@($search.packages)[0].id -ne "elvui") {
        throw "Catalog search did not return ElvUI as the first match."
    }
} finally {
    Pop-Location
}
