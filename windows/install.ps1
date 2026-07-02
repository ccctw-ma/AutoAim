#Requires -Version 5.1

[CmdletBinding()]
param(
    [string]$Repo = "ccctw-ma/AutoAim",
    [Alias("Version")]
    [string]$ReleaseVersion = "latest",
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "AutoAimReview"),
    [string]$PackageAsset = "AutoAimReview-windows-x64.zip",
    [string]$ManifestAsset = "AutoAimReview-windows-x64-manifest.json",
    [switch]$NoPathUpdate,
    [switch]$NoDesktopShortcut,
    [switch]$NoStartMenuShortcut
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Write-Step {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Host "[autoaim] $Message" -ForegroundColor Cyan
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

    Invoke-RestMethod -Headers @{ "User-Agent" = "AutoAimReview-Windows-Installer" } -Uri $uri
}

function Find-ReleaseAsset {
    param(
        [Parameter(Mandatory = $true)]$Release,
        [Parameter(Mandatory = $true)][string]$AssetName
    )

    foreach ($asset in $Release.assets) {
        if ($asset.name -eq $AssetName) {
            return $asset
        }
    }

    throw "Release '$($Release.tag_name)' does not contain asset '$AssetName'."
}

function Get-FileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

function Download-File {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    Invoke-WebRequest -Headers @{ "User-Agent" = "AutoAimReview-Windows-Installer" } -Uri $Uri -OutFile $OutputPath
}

function Add-UserPathEntry {
    param([Parameter(Mandatory = $true)][string]$Path)

    $resolvedPath = (Resolve-Path -Path $Path).Path
    $normalizedPath = $resolvedPath.TrimEnd([char]92)
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = if ([string]::IsNullOrWhiteSpace($currentPath)) { @() } else { $currentPath -split ";" }

    foreach ($part in $parts) {
        if ($part.TrimEnd([char]92) -ieq $normalizedPath) {
            Write-Step "User PATH already contains $resolvedPath"
            return
        }
    }

    $newPath = if ([string]::IsNullOrWhiteSpace($currentPath)) { $resolvedPath } else { "$currentPath;$resolvedPath" }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    if (($env:Path -split ";") -notcontains $resolvedPath) {
        $env:Path = "$env:Path;$resolvedPath"
    }
    Write-Step "Added $resolvedPath to the user PATH. Open a new terminal to use it everywhere."
}

function New-CommandShim {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)][string]$BinDir
    )

    $updateScript = Join-Path $InstallRoot "windows\update.ps1"
    $updateCmd = @"
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$updateScript" -InstallDir "$InstallRoot" %*
"@
    Set-Content -Path (Join-Path $BinDir "autoaim-update.cmd") -Value $updateCmd -Encoding ASCII
}

function New-AppLauncherShim {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)][string]$BinDir
    )

    $guiScript = Join-Path $InstallRoot "windows\AutoAimReview.ps1"
    $launcherCmd = @"
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "$guiScript" %*
"@
    Set-Content -Path (Join-Path $BinDir "autoaim-review.cmd") -Value $launcherCmd -Encoding ASCII
}

function New-Shortcut {
    param(
        [Parameter(Mandatory = $true)][string]$ShortcutPath,
        [Parameter(Mandatory = $true)][string]$TargetPath,
        [Parameter(Mandatory = $true)][string]$WorkingDirectory,
        [Parameter(Mandatory = $true)][string]$Description,
        [string]$IconPath
    )

    New-Item -ItemType Directory -Path (Split-Path -Parent $ShortcutPath) -Force | Out-Null
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($ShortcutPath)
    $shortcut.TargetPath = $TargetPath
    $shortcut.WorkingDirectory = $WorkingDirectory
    $shortcut.Description = $Description
    if ($IconPath -and (Test-Path $IconPath -PathType Leaf)) {
        $shortcut.IconLocation = $IconPath
    }
    $shortcut.Save()
}

function New-ApplicationShortcuts {
    param(
        [Parameter(Mandatory = $true)][string]$InstallRoot,
        [Parameter(Mandatory = $true)][string]$BinDir,
        [bool]$CreateDesktopShortcut,
        [bool]$CreateStartMenuShortcut
    )

    $launcher = Join-Path $BinDir "autoaim-review.cmd"
    $iconPath = Join-Path $InstallRoot "assets\logo.ico"
    if (-not (Test-Path $iconPath -PathType Leaf)) {
        $iconPath = Join-Path $BinDir "autoaim.exe"
    }
    if ($CreateDesktopShortcut) {
        New-Shortcut `
            -ShortcutPath (Join-Path ([Environment]::GetFolderPath("Desktop")) "AutoAim Review.lnk") `
            -TargetPath $launcher `
            -WorkingDirectory $InstallRoot `
            -Description "Open AutoAim Review" `
            -IconPath $iconPath
    }

    if ($CreateStartMenuShortcut) {
        $programs = [Environment]::GetFolderPath("Programs")
        New-Shortcut `
            -ShortcutPath (Join-Path $programs "AutoAim Review\AutoAim Review.lnk") `
            -TargetPath $launcher `
            -WorkingDirectory $InstallRoot `
            -Description "Open AutoAim Review" `
            -IconPath $iconPath
    }
}

function Assert-PackageFiles {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$Root
    )

    foreach ($file in $Manifest.files) {
        $path = Join-Path $Root $file.path
        if (-not (Test-Path $path -PathType Leaf)) {
            throw "Package is missing expected file: $($file.path)"
        }

        $actualSha = Get-FileSha256 $path
        if ($actualSha -ne $file.sha256) {
            throw "Package file hash mismatch for $($file.path). Expected $($file.sha256), got $actualSha."
        }
    }
}

function Copy-PackageToInstallDir {
    param(
        [Parameter(Mandatory = $true)][string]$PackageRoot,
        [Parameter(Mandatory = $true)][string]$TargetRoot
    )

    New-Item -ItemType Directory -Path $TargetRoot -Force | Out-Null
    foreach ($item in Get-ChildItem -Path $PackageRoot -Force) {
        $destination = Join-Path $TargetRoot $item.Name
        if (Test-Path $destination) {
            Remove-Item -Path $destination -Recurse -Force
        }
        Copy-Item -Path $item.FullName -Destination $destination -Recurse -Force
    }
}

$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
$BinDir = Join-Path $InstallDir "bin"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("autoaim-install-" + [Guid]::NewGuid().ToString("N"))

try {
    Write-Step "Resolving AutoAim Review $ReleaseVersion from $Repo"
    $release = Get-GitHubRelease -Repository $Repo -ReleaseVersion $ReleaseVersion
    $packageAssetInfo = Find-ReleaseAsset -Release $release -AssetName $PackageAsset
    $manifestAssetInfo = Find-ReleaseAsset -Release $release -AssetName $ManifestAsset

    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
    $packagePath = Join-Path $tempRoot $PackageAsset
    $manifestPath = Join-Path $tempRoot $ManifestAsset
    $extractDir = Join-Path $tempRoot "package"

    Write-Step "Downloading release manifest"
    Download-File -Uri $manifestAssetInfo.browser_download_url -OutputPath $manifestPath
    $manifest = Get-Content -Path $manifestPath -Raw | ConvertFrom-Json

    Write-Step "Downloading prebuilt Windows package"
    Download-File -Uri $packageAssetInfo.browser_download_url -OutputPath $packagePath

    $actualPackageSha = Get-FileSha256 $packagePath
    if ($manifest.package_asset.sha256 -ne $actualPackageSha) {
        throw "Package hash mismatch. Expected $($manifest.package_asset.sha256), got $actualPackageSha."
    }

    Expand-Archive -Path $packagePath -DestinationPath $extractDir -Force
    Assert-PackageFiles -Manifest $manifest -Root $extractDir

    Write-Step "Installing files into $InstallDir"
    Copy-PackageToInstallDir -PackageRoot $extractDir -TargetRoot $InstallDir
    Copy-Item -Path $manifestPath -Destination (Join-Path $InstallDir "install-manifest.json") -Force
    New-CommandShim -InstallRoot $InstallDir -BinDir $BinDir
    New-AppLauncherShim -InstallRoot $InstallDir -BinDir $BinDir
    New-ApplicationShortcuts `
        -InstallRoot $InstallDir `
        -BinDir $BinDir `
        -CreateDesktopShortcut (-not $NoDesktopShortcut) `
        -CreateStartMenuShortcut (-not $NoStartMenuShortcut)

    if (-not $NoPathUpdate) {
        Add-UserPathEntry -Path $BinDir
    }

    Write-Step "Installation complete"
    Write-Host "Version: $($manifest.version)"
    Write-Host "GUI:    $(Join-Path $BinDir 'autoaim-review.cmd')"
    Write-Host "Binary: $(Join-Path $BinDir 'autoaim.exe')"
    Write-Host "Updater: $(Join-Path $BinDir 'autoaim-update.cmd')"
    Write-Host "Start menu shortcut: AutoAim Review"
    Write-Host "Run 'autoaim-update -CheckOnly' to check for incremental updates."
}
finally {
    if (Test-Path $tempRoot) {
        Remove-Item -Path $tempRoot -Recurse -Force
    }
}
