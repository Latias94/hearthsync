param(
    [string]$WowRetailRoot = 'E:\Games\World of Warcraft\_retail_',
    [string]$NewBeeBoxCacheRoot = 'C:\Program Files\NewBeeBox\NewBeeBoxCache',
    [string]$OutputDir = '',
    [string]$HearthSyncExe = '',
    [switch]$SkipBuild,
    [int]$MaxModuleSamples = 3,
    [string[]]$ModuleSamplePatterns = @(
        '*MeetingStone*.zip',
        '*BigWigs*.zip',
        '*HandyNotes*.zip',
        '*NorthernSkyRaidTools*.zip'
    ),
    [string[]]$ExternalPackageSources = @(),
    [string]$SyntheticSourceAccount = 'SOURCE_ACCOUNT',
    [string]$SyntheticSourceServer = 'SOURCE_REALM',
    [string]$SyntheticSourceCharacter = 'SOURCE_CHARACTER'
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

function Get-SafeCaseId {
    param([string]$Value)

    $safe = $Value -replace '[^A-Za-z0-9_.-]', '-'
    $safe = $safe.Trim('-')
    if ([string]::IsNullOrWhiteSpace($safe)) {
        return 'case'
    }
    return $safe.ToLowerInvariant()
}

function Get-FirstFile {
    param(
        [string]$Root,
        [string]$Filter
    )

    if (-not (Test-Path -LiteralPath $Root)) {
        return $null
    }

    return Get-ChildItem -LiteralPath $Root -Recurse -File -Filter $Filter -Force |
        Sort-Object FullName |
        Select-Object -First 1
}

function Get-ModuleSamples {
    param(
        [string]$Root,
        [int]$Limit,
        [string[]]$PreferredPatterns
    )

    $modulesRoot = Join-Path $Root 'modules'
    if (-not (Test-Path -LiteralPath $modulesRoot)) {
        return @()
    }

    $selected = [System.Collections.Generic.List[object]]::new()
    $selectedPaths = @{}

    foreach ($pattern in $PreferredPatterns) {
        if ($selected.Count -ge $Limit) {
            break
        }

        $match = Get-ChildItem -LiteralPath $modulesRoot -File -Filter $pattern -Force |
            Sort-Object Name |
            Select-Object -First 1
        if ($null -eq $match -or $selectedPaths.ContainsKey($match.FullName)) {
            continue
        }

        $selected.Add($match)
        $selectedPaths[$match.FullName] = $true
    }

    if ($selected.Count -lt $Limit) {
        foreach ($module in Get-ChildItem -LiteralPath $modulesRoot -File -Filter '*.zip' -Force |
        Sort-Object Name |
        Select-Object -First ($Limit * 4)) {
            if ($selected.Count -ge $Limit) {
                break
            }
            if ($selectedPaths.ContainsKey($module.FullName)) {
                continue
            }
            $selected.Add($module)
            $selectedPaths[$module.FullName] = $true
        }
    }

    return @($selected)
}

function Invoke-HearthSyncJson {
    param(
        [string]$Executable,
        [string[]]$Arguments
    )

    $allArgs = @('--json') + $Arguments
    $stderrPath = Join-Path $script:OutputDir '__last-stderr.txt'
    if (Test-Path -LiteralPath $stderrPath) {
        Remove-Item -LiteralPath $stderrPath -Force
    }

    $started = Get-Date
    $stdout = & $Executable @allArgs 2> $stderrPath
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

function Convert-AnalysisSummary {
    param(
        [string]$CaseId,
        [string]$Probe,
        [string]$SourceLabel,
        [object]$Run
    )

    $result = $Run.result
    $summary = $result.summary
    $resources = $result.resources
    $publicSharing = $summary.public_sharing

    return [pscustomobject]@{
        id = $CaseId
        probe = $Probe
        status = if ($Run.exit_code -eq 0) { 'passed' } else { 'failed' }
        source_label = $SourceLabel
        exit_code = $Run.exit_code
        elapsed_ms = $Run.elapsed_ms
        source_kind = $result.source_kind
        layout = $result.layout
        package_id = $result.package_id
        entry_count = $result.entry_count
        total_files = $summary.total_files
        normalized_files = $summary.normalized_files
        ignored_files = $summary.ignored_files
        warning_count = $summary.warning_count
        addons = $summary.addons
        wtf_common = $summary.wtf_common
        wtf_characters = $summary.wtf_characters
        fonts = $summary.fonts
        interface_assets = $summary.interface_assets
        addon_names_sample = @($resources.addons | Select-Object -First 8)
        interface_assets_sample = @($resources.interface_assets | Select-Object -First 8)
        wtf_scopes = @($summary.wtf_scopes | ForEach-Object {
            [pscustomobject]@{
                scope = $_.scope
                risk = $_.risk
                count = $_.count
            }
        })
        sensitive_wtf_files = @($summary.sensitive_wtf_files | ForEach-Object {
            [pscustomobject]@{
                kind = $_.kind
                severity = $_.severity
                count = $_.count
            }
        })
        source_identity_counts = [pscustomobject]@{
            source_accounts = @($summary.source_identities.source_accounts).Count
            source_characters = @($summary.source_identities.source_characters).Count
            entries_with_source_account = $summary.source_identities.entries_with_source_account
            entries_with_source_character = $summary.source_identities.entries_with_source_character
        }
        public_sharing = [pscustomobject]@{
            status = $publicSharing.status
            public_ready = $publicSharing.public_ready
            review_required_count = $publicSharing.review_required_count
            advisory_count = $publicSharing.advisory_count
        }
        error = if ($Run.exit_code -eq 0) { $null } else { $Run.stderr }
    }
}

function Add-InspectionCase {
    param(
        [System.Collections.Generic.List[object]]$Cases,
        [string]$Id,
        [string]$Probe,
        [string]$SourceLabel,
        [string[]]$Arguments
    )

    $Cases.Add([pscustomobject]@{
        id = $Id
        probe = $Probe
        source_label = $SourceLabel
        arguments = $Arguments
    })
}

$repoRoot = Get-RepoRoot
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $OutputDir = Join-Path $repoRoot "target\research\config-package-compatibility-readonly-$timestamp"
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

$cases = [System.Collections.Generic.List[object]]::new()

if (Test-Path -LiteralPath $WowRetailRoot) {
    Add-InspectionCase `
        -Cases $cases `
        -Id 'local-retail-config-tree' `
        -Probe 'config.inspect' `
        -SourceLabel '<local-wow-retail-config-tree>' `
        -Arguments @('config', 'inspect', '--source', $WowRetailRoot)
}

foreach ($module in Get-ModuleSamples `
        -Root $NewBeeBoxCacheRoot `
        -Limit $MaxModuleSamples `
        -PreferredPatterns $ModuleSamplePatterns) {
    Add-InspectionCase `
        -Cases $cases `
        -Id (Get-SafeCaseId -Value "newbeebox-module-$($module.BaseName)") `
        -Probe 'external-package.inspect' `
        -SourceLabel "<newbeebox-module-cache>/$($module.Name)" `
        -Arguments @('external-package', 'inspect', '--source', $module.FullName)
}

$modulesRoot = Join-Path $NewBeeBoxCacheRoot 'modules'
$fontPackage = Get-FirstFile -Root $modulesRoot -Filter 'font-*.zip'
if ($null -ne $fontPackage) {
    Add-InspectionCase `
        -Cases $cases `
        -Id 'newbeebox-font-package' `
        -Probe 'external-package.inspect' `
        -SourceLabel "<newbeebox-module-cache>/$($fontPackage.Name)" `
        -Arguments @('external-package', 'inspect', '--source', $fontPackage.FullName)
}

$materialPackage = Get-FirstFile -Root $modulesRoot -Filter 'material-*.zip'
if ($null -ne $materialPackage) {
    Add-InspectionCase `
        -Cases $cases `
        -Id 'newbeebox-material-package' `
        -Probe 'external-package.inspect' `
        -SourceLabel "<newbeebox-module-cache>/$($materialPackage.Name)" `
        -Arguments @('external-package', 'inspect', '--source', $materialPackage.FullName)
}

foreach ($source in $ExternalPackageSources) {
    if (-not (Test-Path -LiteralPath $source)) {
        Write-Warning "External package source not found: $source"
        continue
    }

    $item = Get-Item -LiteralPath $source
    Add-InspectionCase `
        -Cases $cases `
        -Id (Get-SafeCaseId -Value "external-$($item.BaseName)") `
        -Probe 'external-package.inspect' `
        -SourceLabel "<external-package-source>/$($item.Name)" `
        -Arguments @('external-package', 'inspect', '--source', $item.FullName)
}

$wtfCacheRoot = Join-Path $NewBeeBoxCacheRoot 'wowWtfCache'
$accountWtf = Get-FirstFile -Root $wtfCacheRoot -Filter 'wtfserve-*.zip'
if ($null -ne $accountWtf) {
    Add-InspectionCase `
        -Cases $cases `
        -Id 'newbeebox-wtf-account' `
        -Probe 'external-package.inspect' `
        -SourceLabel "<newbeebox-wtf-cache>/$($accountWtf.Name)" `
        -Arguments @(
            'external-package', 'inspect',
            '--source', $accountWtf.FullName,
            '--source-account', $SyntheticSourceAccount
        )
}

$characterWtf = Get-FirstFile -Root $wtfCacheRoot -Filter 'wtfrole-*.zip'
if ($null -ne $characterWtf) {
    Add-InspectionCase `
        -Cases $cases `
        -Id 'newbeebox-wtf-character' `
        -Probe 'external-package.inspect' `
        -SourceLabel "<newbeebox-wtf-cache>/$($characterWtf.Name)" `
        -Arguments @(
            'external-package', 'inspect',
            '--source', $characterWtf.FullName,
            '--source-account', $SyntheticSourceAccount,
            '--source-server', $SyntheticSourceServer,
            '--source-character', $SyntheticSourceCharacter
        )
}

$results = [System.Collections.Generic.List[object]]::new()
foreach ($case in $cases) {
    Write-Output "Inspecting $($case.id) ..."
    try {
        $run = Invoke-HearthSyncJson -Executable $HearthSyncExe -Arguments $case.arguments
        $results.Add((Convert-AnalysisSummary `
            -CaseId $case.id `
            -Probe $case.probe `
            -SourceLabel $case.source_label `
            -Run $run))
    } catch {
        $results.Add([pscustomobject]@{
            id = $case.id
            probe = $case.probe
            status = 'failed'
            source_label = $case.source_label
            error = $_.Exception.Message
        })
    }
}

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToString('o')
    repo = $repoRoot
    privacy = 'Read-only source inspection. The report stores labels, aggregate counts, layouts, warning/public-sharing summaries, and short addon/interface samples only; it does not store normalized entry lists or file contents.'
    source_roots = [pscustomobject]@{
        wow_retail_root = if (Test-Path -LiteralPath $WowRetailRoot) { '<local-wow-retail-config-tree>' } else { '<missing>' }
        newbeebox_cache_root = if (Test-Path -LiteralPath $NewBeeBoxCacheRoot) { '<newbeebox-cache-root>' } else { '<missing>' }
    }
    cases = $results
}

$jsonPath = Join-Path $OutputDir 'compatibility-readonly-summary.json'
$report | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $jsonPath -Encoding utf8

$mdPath = Join-Path $OutputDir 'compatibility-readonly-summary.md'
$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add('# Config Package Compatibility Read-Only Summary')
$lines.Add('')
$lines.Add("- Generated: $($report.generated_at)")
$lines.Add("- Privacy: $($report.privacy)")
$lines.Add('')
$lines.Add('| Case | Probe | Status | Layout | Entries | Normalized | Warnings | Public sharing |')
$lines.Add('| --- | --- | --- | --- | ---: | ---: | ---: | --- |')
foreach ($case in $results) {
    $layout = if ($null -ne $case.layout) { $case.layout } else { 'n/a' }
    $entries = if ($null -ne $case.entry_count) { $case.entry_count } else { 0 }
    $normalized = if ($null -ne $case.normalized_files) { $case.normalized_files } else { 0 }
    $warnings = if ($null -ne $case.warning_count) { $case.warning_count } else { 0 }
    $sharing = if ($null -ne $case.public_sharing) { $case.public_sharing.status } else { 'n/a' }
    $lines.Add("| $($case.id) | $($case.probe) | $($case.status) | $layout | $entries | $normalized | $warnings | $sharing |")
}
$lines | Set-Content -LiteralPath $mdPath -Encoding utf8

Write-Output "Wrote $jsonPath"
Write-Output "Wrote $mdPath"
