param(
    [string]$CatalogPath = "",
    [switch]$KeepDownloads
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

function Get-WowFlavorFolder {
    param([string]$Flavor)

    switch ($Flavor.ToLowerInvariant()) {
        'retail' { return '_retail_' }
        'classic' { return '_classic_' }
        'classic-era' { return '_classic_era_' }
        'classic_era' { return '_classic_era_' }
        'ptr' { return '_ptr_' }
        'beta' { return '_beta_' }
        'xptr' { return '_xptr_' }
        default {
            throw "unsupported synthetic flavor: $Flavor"
        }
    }
}

function New-SyntheticAddonInstallRoot {
    param(
        [string]$Root,
        [string]$Flavor
    )

    $productRoot = Join-Path $Root 'World of Warcraft'
    $flavorRoot = Join-Path $productRoot (Get-WowFlavorFolder -Flavor $Flavor)
    $interfaceRoot = Join-Path $flavorRoot 'Interface'
    $addonRoot = Join-Path $interfaceRoot 'AddOns'
    $wtfRoot = Join-Path $flavorRoot 'WTF'
    $fontsRoot = Join-Path $flavorRoot 'Fonts'

    foreach ($dir in @($addonRoot, $wtfRoot, $fontsRoot)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }

    return [pscustomobject]@{
        product_root = $productRoot
        flavor_root = $flavorRoot
        addon_root = $addonRoot
    }
}

function Select-ProbeFlavor {
    param([object]$Package)

    $supportedFlavors = @($Package.supported_flavors)
    foreach ($preferred in @('retail', 'classic-era', 'classic', 'ptr', 'beta', 'xptr')) {
        if ($supportedFlavors -contains $preferred) {
            return $preferred
        }
    }

    if ($supportedFlavors.Count -gt 0) {
        return [string]$supportedFlavors[0]
    }

    return 'retail'
}

function New-AddonDryRunArguments {
    param(
        [string]$InstallRoot,
        [string]$CacheRoot,
        [string]$Flavor,
        [string]$Source
    )

    return @(
        '--json',
        '--addon-state-storage', 'sidecar',
        '--addon-cache-dir', $CacheRoot,
        'addon',
        'install',
        '--install', $InstallRoot,
        '--flavor', $Flavor,
        '--source', $Source,
        '--dry-run',
        '--replace-existing'
    )
}

function Invoke-HearthSyncJson {
    param(
        [string[]]$Arguments
    )

    $stderrPath = Join-Path $script:ValidationRoot '__last-stderr.txt'
    if (Test-Path -LiteralPath $stderrPath) {
        Remove-Item -LiteralPath $stderrPath -Force
    }

    $started = Get-Date
    $stdout = & cargo run --quiet -- @Arguments 2> $stderrPath
    $exitCode = $LASTEXITCODE
    $elapsedMs = [int]((Get-Date) - $started).TotalMilliseconds
    $stdoutText = ($stdout | Out-String).Trim()
    $stderrText = ''
    if (Test-Path -LiteralPath $stderrPath) {
        $stderrRaw = Get-Content -LiteralPath $stderrPath -Raw
        if ($null -ne $stderrRaw) {
            $stderrText = $stderrRaw.Trim()
        }
    }

    $result = $null
    if ($exitCode -eq 0 -and -not [string]::IsNullOrWhiteSpace($stdoutText)) {
        try {
            $result = $stdoutText | ConvertFrom-Json
        } catch {
            $result = $null
        }
    }

    return [pscustomobject]@{
        exit_code = $exitCode
        elapsed_ms = $elapsedMs
        result = $result
        stdout = $stdoutText
        stderr = $stderrText
    }
}

Push-Location $RepoRoot
$validationSucceeded = $false
try {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $script:ValidationRoot = Join-Path $RepoRoot "target\research\catalog-readonly-validation-$timestamp"
    New-Item -ItemType Directory -Force -Path $script:ValidationRoot | Out-Null
    $cacheRoot = Join-Path $script:ValidationRoot 'download-cache'

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

    foreach ($package in @($inspect.packages)) {
        $probeFlavor = Select-ProbeFlavor -Package $package
        $syntheticRoot = Join-Path $script:ValidationRoot "synthetic-install-$($package.id)"
        $synthetic = New-SyntheticAddonInstallRoot -Root $syntheticRoot -Flavor $probeFlavor
        $arguments = New-AddonDryRunArguments `
            -InstallRoot $synthetic.product_root `
            -CacheRoot $cacheRoot `
            -Flavor $probeFlavor `
            -Source $package.source_label

        Write-Output "Read-only catalog provider dry-run $($package.id) ($probeFlavor) ..."
        $run = Invoke-HearthSyncJson -Arguments $arguments

        if ($run.exit_code -ne 0 -or $null -eq $run.result) {
            $detail = if ([string]::IsNullOrWhiteSpace($run.stderr)) {
                $run.stdout
            } else {
                $run.stderr
            }
            throw "Catalog live probe failed for package `"$($package.id)`" using source `"$($package.source_label)`": $detail"
        }

        if ($run.result.source_label -ne $package.source_label) {
            throw "Catalog live probe source label mismatch for `"$($package.id)`": expected `"$($package.source_label)`", got `"$($run.result.source_label)`""
        }
    }

    if (-not $KeepDownloads -and (Test-Path -LiteralPath $cacheRoot)) {
        Remove-Item -LiteralPath $cacheRoot -Recurse -Force
    }
    $validationSucceeded = $true
} finally {
    Pop-Location
    if ($validationSucceeded -and -not $KeepDownloads -and (Test-Path -LiteralPath $script:ValidationRoot)) {
        Remove-Item -LiteralPath $script:ValidationRoot -Recurse -Force
    }
}
