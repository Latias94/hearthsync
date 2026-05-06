param(
    [string]$CatalogPath = ""
)

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($CatalogPath)) {
    $CatalogPath = Join-Path $RepoRoot "catalog\community-addon-index.toml"
}
$CatalogDirectory = Split-Path -Parent $CatalogPath
$CatalogBaseName = [System.IO.Path]::GetFileNameWithoutExtension($CatalogPath)
$GovernancePath = Join-Path $CatalogDirectory "$CatalogBaseName.governance.json"

function Assert-NonEmptyString {
    param(
        [string]$Value,
        [string]$FieldName
    )

    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw "$FieldName must not be empty."
    }
}

function Assert-StringArray {
    param(
        [object]$Values,
        [string]$FieldName,
        [string]$ItemLabel
    )

    $items = @($Values)
    if ($items.Count -lt 1) {
        throw "$FieldName must contain at least one $ItemLabel."
    }

    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($item in $items) {
        Assert-NonEmptyString -Value ([string]$item) -FieldName $ItemLabel
        if (-not $seen.Add(([string]$item).Trim())) {
            throw "$FieldName contains duplicate value: $item"
        }
    }
}

function Assert-GovernanceEntry {
    param([object]$Entry)

    Assert-NonEmptyString -Value ([string]$Entry.id) -FieldName 'Governance entry id'
    Assert-StringArray -Values $Entry.aliases -FieldName "Governance entry `"$($Entry.id)`" aliases" -ItemLabel 'alias'
    Assert-StringArray -Values $Entry.upstream_hosts -FieldName "Governance entry `"$($Entry.id)`" upstream hosts" -ItemLabel 'upstream host'
    foreach ($upstreamHost in @($Entry.upstream_hosts)) {
        if ([string]$upstreamHost -notmatch '^[a-z0-9_-]+$') {
            throw "Governance entry `"$($Entry.id)`" upstream host must contain only lowercase ASCII letters, digits, `-`, or `_`: $upstreamHost"
        }
    }

    Assert-NonEmptyString -Value ([string]$Entry.source_attribution) -FieldName "Governance entry `"$($Entry.id)`" source_attribution"
    if ($null -ne $Entry.maintainer) {
        Assert-NonEmptyString -Value ([string]$Entry.maintainer) -FieldName "Governance entry `"$($Entry.id)`" maintainer"
    }

    if (@('active', 'legacy', 'archived', 'blocked') -notcontains [string]$Entry.status) {
        throw "Governance entry `"$($Entry.id)`" has unsupported status: $($Entry.status)"
    }

    if (@('high', 'medium', 'low') -notcontains [string]$Entry.confidence) {
        throw "Governance entry `"$($Entry.id)`" has unsupported confidence: $($Entry.confidence)"
    }

    Assert-NonEmptyString -Value ([string]$Entry.last_verified_at) -FieldName "Governance entry `"$($Entry.id)`" last_verified_at"
    try {
        [DateTimeOffset]::Parse([string]$Entry.last_verified_at) | Out-Null
    } catch {
        throw "Governance entry `"$($Entry.id)`" last_verified_at is not valid ISO-8601 date/time: $($Entry.last_verified_at)"
    }

    if ($null -ne $Entry.notes) {
        Assert-NonEmptyString -Value ([string]$Entry.notes) -FieldName "Governance entry `"$($Entry.id)`" notes"
    }
}

function Assert-GovernanceFile {
    param(
        [object]$Governance,
        [object[]]$IndexPackages
    )

    if ($Governance.schema_version -ne 1) {
        throw "Unsupported governance schema version: $($Governance.schema_version)"
    }

    Assert-NonEmptyString -Value ([string]$Governance.name) -FieldName 'Governance name'
    Assert-NonEmptyString -Value ([string]$Governance.updated_at) -FieldName 'Governance updated_at'
    try {
        [DateTimeOffset]::Parse([string]$Governance.updated_at) | Out-Null
    } catch {
        throw "Governance updated_at is not valid ISO-8601 date/time: $($Governance.updated_at)"
    }

    $entries = @($Governance.entries)
    if ($entries.Count -lt 1) {
        throw 'Governance file must contain at least one entry.'
    }

    $indexPackageIds = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($package in @($IndexPackages)) {
        [void]$indexPackageIds.Add([string]$package.id)
    }

    $governancePackageIds = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in $entries) {
        Assert-GovernanceEntry -Entry $entry
        [void]$governancePackageIds.Add([string]$entry.id)
    }

    foreach ($packageId in $indexPackageIds) {
        if (-not $governancePackageIds.Contains($packageId)) {
            throw "Governance file is missing an entry for catalog package id: $packageId"
        }
    }

    foreach ($packageId in $governancePackageIds) {
        if (-not $indexPackageIds.Contains($packageId)) {
            throw "Governance file contains an unknown package id: $packageId"
        }
    }
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

    if (-not (Test-Path $GovernancePath)) {
        throw "Catalog governance file is missing: $GovernancePath"
    }

    $governanceJson = Get-Content $GovernancePath -Raw | ConvertFrom-Json
    Assert-GovernanceFile -Governance $governanceJson -IndexPackages @($inspect.packages)

    $aliasSearchJson = cargo run --quiet -- --json addon index search --file $CatalogPath --query 'Big Wigs' --limit 5 | Out-String
    $aliasSearch = $aliasSearchJson | ConvertFrom-Json

    if ($aliasSearch.returned_package_count -lt 1) {
        throw "Catalog alias search did not return any packages for Big Wigs."
    }

    if (@($aliasSearch.packages)[0].id -ne "bigwigs") {
        throw "Catalog alias search did not return BigWigs as the first match."
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
