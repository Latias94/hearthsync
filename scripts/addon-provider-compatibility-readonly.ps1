param(
    [string]$OutputDir = '',
    [string]$HearthSyncExe = '',
    [switch]$SkipBuild,
    [string]$SyntheticFlavor = 'retail',
    [string]$GitHubSource = 'github:BigWigsMods/BigWigs@v414.9#BigWigs-v414.9.zip',
    [string]$WagoSource = 'wago:qv63A7Gb@vdx1042w',
    [string]$HttpSource = 'https://github.com/BigWigsMods/BigWigs/releases/download/v414.9/BigWigs-v414.9.zip',
    [string]$ExtraHttpSource = 'https://sourceforge.net/projects/elvui.mirror/files/6.09/6.09%20source%20code.zip/download',
    [string]$TukuiSource = 'tukui:elvui',
    [string]$CurseForgeSource = 'curseforge:238222',
    [switch]$IncludeCurseForge,
    [switch]$KeepDownloads
)

$ErrorActionPreference = 'Stop'

function Get-RepoRoot {
    $scriptRoot = Split-Path -Parent $PSCommandPath
    return (Resolve-Path -LiteralPath (Join-Path $scriptRoot '..')).Path
}

function Get-DefaultExecutablePath {
    param([string]$RepoRoot)

    if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
            [System.Runtime.InteropServices.OSPlatform]::Windows
        )) {
        return Join-Path $RepoRoot 'target\debug\hearthsync.exe'
    }

    return Join-Path $RepoRoot 'target/debug/hearthsync'
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

function Invoke-HearthSyncJson {
    param(
        [string]$Executable,
        [string[]]$Arguments
    )

    $stderrPath = Join-Path $script:OutputDir '__last-stderr.txt'
    if (Test-Path -LiteralPath $stderrPath) {
        Remove-Item -LiteralPath $stderrPath -Force
    }

    $started = Get-Date
    $stdout = & $Executable @Arguments 2> $stderrPath
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
        $result = $stdoutText | ConvertFrom-Json
    }

    return [pscustomobject]@{
        exit_code = $exitCode
        elapsed_ms = $elapsedMs
        result = $result
        stderr = $stderrText
    }
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

function Convert-AddonDryRunSummary {
    param(
        [string]$Id,
        [string]$Provider,
        [string]$Source,
        [object]$Run
    )

    if ($Run.exit_code -ne 0 -or $null -eq $Run.result) {
        return [pscustomobject]@{
            id = $Id
            provider = $Provider
            status = 'failed'
            source = $Source
            exit_code = $Run.exit_code
            elapsed_ms = $Run.elapsed_ms
            package_id = $null
            addon_count = 0
            files_to_write = 0
            written_files = 0
            error = $Run.stderr
        }
    }

    return [pscustomobject]@{
        id = $Id
        provider = $Provider
        status = 'passed'
        source = $Source
        exit_code = $Run.exit_code
        elapsed_ms = $Run.elapsed_ms
        package_id = $Run.result.package_id
        addon_count = @($Run.result.addons).Count
        files_to_write = $Run.result.files_to_write
        written_files = $Run.result.written_files
        source_kind = $Run.result.source.kind
        source_label = $Run.result.source_label
        error = $null
    }
}

function Add-Case {
    param(
        [System.Collections.Generic.List[object]]$Cases,
        [string]$Id,
        [string]$Provider,
        [string]$Source,
        [bool]$Enabled,
        [string]$SkipReason
    )

    $Cases.Add([pscustomobject]@{
        id = $Id
        provider = $Provider
        source = $Source
        enabled = $Enabled
        skip_reason = $SkipReason
    })
}

$repoRoot = Get-RepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $OutputDir = Join-Path $repoRoot "target\research\addon-provider-compatibility-readonly-$timestamp"
}
$script:OutputDir = $OutputDir
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

if ([string]::IsNullOrWhiteSpace($HearthSyncExe)) {
    $HearthSyncExe = Get-DefaultExecutablePath -RepoRoot $repoRoot
    if (-not $SkipBuild) {
        Push-Location $repoRoot
        try {
            cargo build --quiet --bin hearthsync
        } finally {
            Pop-Location
        }
    }
}

if (-not (Test-Path -LiteralPath $HearthSyncExe)) {
    throw "hearthsync executable not found: $HearthSyncExe"
}

$synthetic = New-SyntheticAddonInstallRoot `
    -Root (Join-Path $OutputDir 'synthetic-install') `
    -Flavor $SyntheticFlavor
$cacheRoot = Join-Path $OutputDir 'download-cache'

$cases = [System.Collections.Generic.List[object]]::new()
Add-Case -Cases $cases -Id 'http-github-release-asset' -Provider 'http' -Source $HttpSource -Enabled $true -SkipReason ''
Add-Case -Cases $cases -Id 'http-sourceforge-elvui-archive' -Provider 'http' -Source $ExtraHttpSource -Enabled $true -SkipReason ''
Add-Case -Cases $cases -Id 'github-release-exact-asset' -Provider 'github' -Source $GitHubSource -Enabled $true -SkipReason ''
Add-Case -Cases $cases -Id 'wago-exact-release' -Provider 'wago' -Source $WagoSource -Enabled $true -SkipReason ''
Add-Case -Cases $cases -Id 'tukui-elvui-latest' -Provider 'tukui' -Source $TukuiSource -Enabled $true -SkipReason ''

$curseForgeKeyPresent = -not [string]::IsNullOrWhiteSpace($env:HEARTHSYNC_CURSEFORGE_API_KEY) -or
    -not [string]::IsNullOrWhiteSpace($env:CURSEFORGE_API_KEY)
Add-Case `
    -Cases $cases `
    -Id 'curseforge-mod-latest' `
    -Provider 'curseforge' `
    -Source $CurseForgeSource `
    -Enabled ([bool]($IncludeCurseForge -and $curseForgeKeyPresent)) `
    -SkipReason 'requires -IncludeCurseForge and HEARTHSYNC_CURSEFORGE_API_KEY or CURSEFORGE_API_KEY'

$results = [System.Collections.Generic.List[object]]::new()
foreach ($case in $cases) {
    if (-not $case.enabled) {
        $results.Add([pscustomobject]@{
            id = $case.id
            provider = $case.provider
            status = 'skipped'
            source = $case.source
            skip_reason = $case.skip_reason
        })
        continue
    }

    Write-Output "Read-only addon provider dry-run $($case.id) ..."
    try {
        $arguments = New-AddonDryRunArguments `
            -InstallRoot $synthetic.product_root `
            -CacheRoot $cacheRoot `
            -Flavor $SyntheticFlavor `
            -Source $case.source
        $run = Invoke-HearthSyncJson -Executable $HearthSyncExe -Arguments $arguments
        $results.Add((Convert-AddonDryRunSummary `
            -Id $case.id `
            -Provider $case.provider `
            -Source $case.source `
            -Run $run))
    } catch {
        $results.Add([pscustomobject]@{
            id = $case.id
            provider = $case.provider
            status = 'failed'
            source = $case.source
            error = $_.Exception.Message
        })
    }
}

if (-not $KeepDownloads -and (Test-Path -LiteralPath $cacheRoot)) {
    Remove-Item -LiteralPath $cacheRoot -Recurse -Force
}

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToString('o')
    repo = $repoRoot
    privacy = 'Read-only provider compatibility run. It uses addon install --dry-run against a synthetic WoW installation under target/research, records aggregate result fields only, and never mutates a real installation. Provider downloads may touch remote services and the temporary download cache.'
    synthetic_install = [pscustomobject]@{
        product_root = '<synthetic-wow-product-root>'
        flavor = $SyntheticFlavor
        addon_root = '<synthetic-addon-root>'
    }
    downloads_retained = [bool]$KeepDownloads
    cases = $results
}

$jsonPath = Join-Path $OutputDir 'addon-provider-compatibility-readonly-summary.json'
$report | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $jsonPath -Encoding utf8

$mdPath = Join-Path $OutputDir 'addon-provider-compatibility-readonly-summary.md'
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add('# Addon Provider Compatibility Read-Only Summary')
$lines.Add('')
$lines.Add("- Generated: $($report.generated_at)")
$lines.Add("- Privacy: $($report.privacy)")
$lines.Add('')
$lines.Add('| Case | Provider | Status | Package | Addons | Files to write | Written | Error / skip |')
$lines.Add('| --- | --- | --- | --- | ---: | ---: | ---: | --- |')
foreach ($case in $results) {
    $package = if ($null -ne $case.package_id) { $case.package_id } else { 'n/a' }
    $addons = if ($null -ne $case.addon_count) { $case.addon_count } else { 0 }
    $files = if ($null -ne $case.files_to_write) { $case.files_to_write } else { 0 }
    $written = if ($null -ne $case.written_files) { $case.written_files } else { 0 }
    $detail = if ($null -ne $case.error) {
        $case.error
    } elseif ($null -ne $case.skip_reason) {
        $case.skip_reason
    } else {
        ''
    }
    $detail = ($detail -replace '\|', '/')
    $lines.Add("| $($case.id) | $($case.provider) | $($case.status) | $package | $addons | $files | $written | $detail |")
}
$lines | Set-Content -LiteralPath $mdPath -Encoding utf8

Write-Output "Wrote $jsonPath"
Write-Output "Wrote $mdPath"
