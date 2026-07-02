#Requires -Version 5.1

[CmdletBinding()]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "AutoAimReview"),
    [string]$Repo = "ccctw-ma/AutoAim",
    [string]$TargetVersion = "latest",
    [switch]$CheckOnly,
    [switch]$ShowDiff,
    [switch]$AllowFullPackageFallback
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Write-Step {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Host "[autoaim] $Message" -ForegroundColor Cyan
}

function Download-File {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    Invoke-WebRequest -Headers @{ "User-Agent" = "AutoAimReview-Windows-Updater" } -Uri $Uri -OutFile $OutputPath
}

function Get-GitHubRelease {
    param(
        [Parameter(Mandatory = $true)][string]$Repository,
        [Parameter(Mandatory = $true)][string]$ReleaseVersion
    )

    $uri = if ($ReleaseVersion -eq "latest") {
        "https://api.github.com/repos/$Repository/releases/latest"
    }
    else {
        "https://api.github.com/repos/$Repository/releases/tags/$ReleaseVersion"
    }

    Invoke-RestMethod -Headers @{ "User-Agent" = "AutoAimReview-Windows-Updater" } -Uri $uri
}

function Find-ReleaseAsset {
    param(
        [Parameter(Mandatory = $true)]$Release,
        [Parameter(Mandatory = $true)][string]$AssetName,
        [bool]$Required = $true
    )

    foreach ($asset in $Release.assets) {
        if ($asset.name -eq $AssetName) {
            return $asset
        }
    }

    if ($Required) {
        throw "Release '$($Release.tag_name)' does not contain asset '$AssetName'."
    }
    return $null
}

function Get-FileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

function Read-InstallManifest {
    param([Parameter(Mandatory = $true)][string]$InstallRoot)

    $path = Join-Path $InstallRoot "install-manifest.json"
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Install manifest not found at $path. Reinstall from a released Windows package first."
    }

    Get-Content -Path $path -Raw | ConvertFrom-Json
}

function Find-DeltaEntry {
    param(
        [Parameter(Mandatory = $true)]$Index,
        [Parameter(Mandatory = $true)][string]$FromVersion,
        [Parameter(Mandatory = $true)][string]$ToVersion
    )

    foreach ($delta in $Index.deltas) {
        if ($delta.from_version -eq $FromVersion -and $delta.to_version -eq $ToVersion) {
            return $delta
        }
    }

    return $null
}

function Assert-CurrentFile {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)]$File
    )

    $path = Join-Path $InstallRoot $File.path
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Cannot apply incremental update because local file is missing: $($File.path)"
    }

    $actual = Get-FileSha256 $path
    if ($actual -ne $File.from_sha256) {
        throw "Cannot apply incremental update because local file changed: $($File.path). Expected $($File.from_sha256), got $actual."
    }
}

function Write-DeltaSummary {
    param([Parameter(Mandatory = $true)]$Delta)

    Write-Step "Incremental update available"
    Write-Host "From: $($Delta.from_version)"
    Write-Host "To:   $($Delta.to_version)"
    if ($Delta.PSObject.Properties.Name -contains "summary") {
        Write-Host "Files changed: $($Delta.summary.changed_files)"
        Write-Host "Files added:   $($Delta.summary.added_files)"
        Write-Host "Files removed: $($Delta.summary.removed_files)"
        Write-Host "Patch bytes:   $($Delta.summary.patch_bytes)"
    }

    Write-Host ""
    Write-Host "Changed paths:"
    foreach ($file in $Delta.files) {
        Write-Host ("{0}`t{1}" -f $file.action, $file.path)
    }

    if ($ShowDiff -and ($Delta.PSObject.Properties.Name -contains "notes")) {
        Write-Host ""
        Write-Host "Release notes:"
        Write-Host $Delta.notes
    }
}

function Decode-Base64ToFile {
    param(
        [Parameter(Mandatory = $true)][string]$Base64,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    $bytes = [Convert]::FromBase64String($Base64)
    New-Item -ItemType Directory -Path (Split-Path -Parent $OutputPath) -Force | Out-Null
    [IO.File]::WriteAllBytes($OutputPath, $bytes)
}

function Apply-BlockPatch {
    param(
        [Parameter(Mandatory = $true)][string]$SourcePath,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [Parameter(Mandatory = $true)]$File
    )

    New-Item -ItemType Directory -Path (Split-Path -Parent $OutputPath) -Force | Out-Null
    Copy-Item -Path $SourcePath -Destination $OutputPath -Force

    $stream = [IO.File]::Open($OutputPath, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
    try {
        foreach ($block in $File.patch.blocks) {
            $bytes = [Convert]::FromBase64String($block.data_base64)
            $stream.Position = [int64]$block.offset
            $stream.Write($bytes, 0, $bytes.Length)
        }
        $stream.SetLength([int64]$File.to_size)
    }
    finally {
        $stream.Dispose()
    }
}

function Apply-Delta {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)]$Delta,
        [Parameter(Mandatory = $true)][string]$ManifestPath
    )

    $stageRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("autoaim-update-stage-" + [Guid]::NewGuid().ToString("N"))
    $backupRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("autoaim-update-backup-" + [Guid]::NewGuid().ToString("N"))

    try {
        New-Item -ItemType Directory -Path $stageRoot -Force | Out-Null
        New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null

        foreach ($file in $Delta.files) {
            if ($file.action -ne "add") {
                Assert-CurrentFile -InstallRoot $InstallRoot -File $file
            }

            if ($file.action -eq "add") {
                $stagedPath = Join-Path $stageRoot $file.path
                Decode-Base64ToFile -Base64 $file.content_base64 -OutputPath $stagedPath
                $actual = Get-FileSha256 $stagedPath
                if ($actual -ne $file.to_sha256) {
                    throw "Downloaded delta content hash mismatch for $($file.path). Expected $($file.to_sha256), got $actual."
                }
            }
            elseif ($file.action -eq "patch") {
                $targetPath = Join-Path $InstallRoot $file.path
                $stagedPath = Join-Path $stageRoot $file.path
                Apply-BlockPatch -SourcePath $targetPath -OutputPath $stagedPath -File $file
                $actual = Get-FileSha256 $stagedPath
                if ($actual -ne $file.to_sha256) {
                    throw "Patched content hash mismatch for $($file.path). Expected $($file.to_sha256), got $actual."
                }
            }
        }

        foreach ($file in $Delta.files) {
            $targetPath = Join-Path $InstallRoot $file.path
            $backupPath = Join-Path $backupRoot $file.path

            if (Test-Path $targetPath -PathType Leaf) {
                New-Item -ItemType Directory -Path (Split-Path -Parent $backupPath) -Force | Out-Null
                Copy-Item -Path $targetPath -Destination $backupPath -Force
            }

            if ($file.action -eq "remove") {
                if (Test-Path $targetPath -PathType Leaf) {
                    Remove-Item -Path $targetPath -Force
                }
            }
            elseif ($file.action -eq "add" -or $file.action -eq "patch") {
                New-Item -ItemType Directory -Path (Split-Path -Parent $targetPath) -Force | Out-Null
                Copy-Item -Path (Join-Path $stageRoot $file.path) -Destination $targetPath -Force
            }
            else {
                throw "Unsupported delta action '$($file.action)' for $($file.path)."
            }
        }

        Copy-Item -Path $ManifestPath -Destination (Join-Path $InstallRoot "install-manifest.json") -Force
    }
    catch {
        Write-Warning "Incremental update failed. Restoring changed files from backup."
        foreach ($backupFile in Get-ChildItem -Path $backupRoot -File -Recurse -ErrorAction SilentlyContinue) {
            $relative = $backupFile.FullName.Substring($backupRoot.Length).TrimStart([char]92, [char]47)
            $targetPath = Join-Path $InstallRoot $relative
            New-Item -ItemType Directory -Path (Split-Path -Parent $targetPath) -Force | Out-Null
            Copy-Item -Path $backupFile.FullName -Destination $targetPath -Force
        }
        throw
    }
    finally {
        if (Test-Path $stageRoot) {
            Remove-Item -Path $stageRoot -Recurse -Force
        }
        if (Test-Path $backupRoot) {
            Remove-Item -Path $backupRoot -Recurse -Force
        }
    }
}

$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("autoaim-update-" + [Guid]::NewGuid().ToString("N"))

try {
    $currentManifest = Read-InstallManifest -InstallRoot $InstallDir
    Write-Step "Checking AutoAim Review updates"
    Write-Host "Installed version: $($currentManifest.version)"

    $release = Get-GitHubRelease -Repository $Repo -ReleaseVersion $TargetVersion
    $targetVersionValue = $release.tag_name
    Write-Host "Latest version:    $targetVersionValue"

    if ($currentManifest.version -eq $targetVersionValue) {
        Write-Step "Already up to date"
        return
    }

    $indexName = "AutoAimReview-windows-x64-deltas.json"
    $indexAsset = Find-ReleaseAsset -Release $release -AssetName $indexName -Required $false
    if ($null -eq $indexAsset) {
        if ($AllowFullPackageFallback) {
            throw "Full package fallback is not implemented in this updater yet. Install the target release manually."
        }
        throw "No incremental update index found in release '$targetVersionValue'. Full-package replacement was not used."
    }

    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
    $indexPath = Join-Path $tempRoot $indexName
    Download-File -Uri $indexAsset.browser_download_url -OutputPath $indexPath
    $index = Get-Content -Path $indexPath -Raw | ConvertFrom-Json
    $delta = Find-DeltaEntry -Index $index -FromVersion $currentManifest.version -ToVersion $targetVersionValue

    if ($null -eq $delta) {
        throw "No incremental delta is available from $($currentManifest.version) to $targetVersionValue. Full-package replacement was not used."
    }

    Write-DeltaSummary -Delta $delta
    if ($CheckOnly) {
        Write-Step "Check only; no files were updated"
        return
    }

    $deltaAsset = Find-ReleaseAsset -Release $release -AssetName $delta.asset
    $targetManifestAsset = Find-ReleaseAsset -Release $release -AssetName "AutoAimReview-windows-x64-manifest.json"
    $deltaPath = Join-Path $tempRoot $delta.asset
    $targetManifestPath = Join-Path $tempRoot "target-manifest.json"

    Write-Step "Downloading incremental patch"
    Download-File -Uri $deltaAsset.browser_download_url -OutputPath $deltaPath
    $actualDeltaSha = Get-FileSha256 $deltaPath
    if ($actualDeltaSha -ne $delta.sha256) {
        throw "Delta hash mismatch. Expected $($delta.sha256), got $actualDeltaSha."
    }

    Download-File -Uri $targetManifestAsset.browser_download_url -OutputPath $targetManifestPath
    $deltaPayload = Get-Content -Path $deltaPath -Raw | ConvertFrom-Json

    Write-Step "Applying incremental patch"
    Apply-Delta -InstallRoot $InstallDir -Delta $deltaPayload -ManifestPath $targetManifestPath
    Write-Step "Update complete"
    Write-Host "Installed version: $targetVersionValue"
}
finally {
    if (Test-Path $tempRoot) {
        Remove-Item -Path $tempRoot -Recurse -Force
    }
}
