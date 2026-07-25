param(
  [string]$ExpectedVersion = "0.2.0"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Read-CargoVersion([string]$Path) {
  $match = [regex]::Match((Get-Content -Raw -LiteralPath $Path), '(?m)^version\s*=\s*"([^"]+)"')
  if (-not $match.Success) { throw "version not found in $Path" }
  return $match.Groups[1].Value
}

function Read-CargoLockVersion([string]$Path, [string]$PackageName) {
  $content = Get-Content -Raw -LiteralPath $Path
  $pattern = '(?ms)\[\[package\]\]\s*name\s*=\s*"' +
    [regex]::Escape($PackageName) +
    '"\s*version\s*=\s*"([^"]+)"'
  $match = [regex]::Match($content, $pattern)
  if (-not $match.Success) { throw "package $PackageName not found in $Path" }
  return $match.Groups[1].Value
}

function Read-NpmLockVersion([string]$Path) {
  $match = [regex]::Match(
    (Get-Content -Raw -LiteralPath $Path),
    '(?m)^\s{2}"version"\s*:\s*"([^"]+)"'
  )
  if (-not $match.Success) { throw "root version not found in $Path" }
  return $match.Groups[1].Value
}

function Get-Sha256([string]$Path) {
  $stream = [IO.File]::OpenRead($Path)
  try {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    try {
      return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace("-", "").ToLowerInvariant()
    } finally {
      $algorithm.Dispose()
    }
  } finally {
    $stream.Dispose()
  }
}

$packageVersion = (Get-Content -Raw -LiteralPath "package.json" | ConvertFrom-Json).version
$frontendVersion = (Get-Content -Raw -LiteralPath "frontend\package.json" | ConvertFrom-Json).version
$rootLockVersion = Read-NpmLockVersion "package-lock.json"
$frontendLockVersion = Read-NpmLockVersion "frontend\package-lock.json"
$tauriVersion = (Get-Content -Raw -LiteralPath "src-tauri\tauri.conf.json" | ConvertFrom-Json).version
$desktopVersion = Read-CargoVersion "src-tauri\Cargo.toml"
$mcpVersion = Read-CargoVersion "forgeos-mcp\Cargo.toml"
$desktopLockVersion = Read-CargoLockVersion "src-tauri\Cargo.lock" "forgeos"
$mcpLockVersion = Read-CargoLockVersion "forgeos-mcp\Cargo.lock" "forgeos-mcp"
$versions = @(
  $packageVersion,
  $frontendVersion,
  $rootLockVersion,
  $frontendLockVersion,
  $tauriVersion,
  $desktopVersion,
  $mcpVersion,
  $desktopLockVersion,
  $mcpLockVersion
)
$unexpectedVersions = @($versions | Where-Object { $_ -ne $ExpectedVersion })
if ($unexpectedVersions.Count -gt 0) {
  throw "release versions must all equal $ExpectedVersion; found: $($versions -join ', ')"
}

Write-Host "Building ForgeOS v$packageVersion Windows installer..."

$nsisOutput = Join-Path $root "src-tauri\target\release\bundle\nsis"
if (Test-Path -LiteralPath $nsisOutput) {
  $resolvedNsis = (Resolve-Path -LiteralPath $nsisOutput).Path
  $targetRoot = [IO.Path]::GetFullPath((Join-Path $root "src-tauri\target"))
  if (-not $resolvedNsis.StartsWith($targetRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "NSIS output escaped the Tauri target directory"
  }
  Remove-Item -LiteralPath $nsisOutput -Recurse -Force
}

npm run tauri:build
if ($LASTEXITCODE -ne 0) { throw "Tauri release build failed" }

$sidecarSource = Join-Path $root "src-tauri\binaries\forgeos-mcp-x86_64-pc-windows-msvc.exe"
$sidecarStaged = Join-Path $root "src-tauri\target\release\forgeos-mcp.exe"
if (-not (Test-Path -LiteralPath $sidecarSource -PathType Leaf) -or
    -not (Test-Path -LiteralPath $sidecarStaged -PathType Leaf)) {
  throw "Tauri did not stage the forgeos-mcp sidecar"
}
$sourceSidecarHash = Get-Sha256 $sidecarSource
$stagedSidecarHash = Get-Sha256 $sidecarStaged
if ($sourceSidecarHash -ne $stagedSidecarHash) {
  throw "the Sidecar staged for NSIS does not match the freshly built forgeos-mcp executable"
}
Write-Host "Verified staged Sidecar SHA-256: $($stagedSidecarHash.ToLowerInvariant())"

$installer = Get-ChildItem -Path "src-tauri\target\release\bundle\nsis" -Filter "*.exe" |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1
if (-not $installer) { throw "NSIS installer was not produced" }

$outputDirectory = Join-Path $root "dist-release"
if (Test-Path -LiteralPath $outputDirectory) {
  $previousArtifact = Join-Path $outputDirectory "ForgeOS-0.1.0-windows-x64-setup.exe"
  $previousChecksums = Join-Path $outputDirectory "SHA256SUMS.txt"
  if ($ExpectedVersion -ne "0.1.0" -and
      (Test-Path -LiteralPath $previousArtifact -PathType Leaf) -and
      (Test-Path -LiteralPath $previousChecksums -PathType Leaf)) {
    $fixtureDirectory = Join-Path $root "src-tauri\target\release-fixtures\v0.1.0"
    New-Item -ItemType Directory -Force -Path $fixtureDirectory | Out-Null
    Copy-Item -LiteralPath $previousArtifact -Destination $fixtureDirectory -Force
    Copy-Item -LiteralPath $previousChecksums -Destination $fixtureDirectory -Force
    Write-Host "Preserved the v0.1.0 installer for the upgrade acceptance test."
  }
  $resolved = (Resolve-Path -LiteralPath $outputDirectory).Path
  if (-not $resolved.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "release output escaped the workspace"
  }
  Remove-Item -LiteralPath $outputDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDirectory | Out-Null

$artifactName = "ForgeOS-$packageVersion-windows-x64-setup.exe"
$artifact = Join-Path $outputDirectory $artifactName
Copy-Item -LiteralPath $installer.FullName -Destination $artifact
$hash = Get-Sha256 $artifact
Set-Content -LiteralPath (Join-Path $outputDirectory "SHA256SUMS.txt") -Encoding ascii -Value "$hash  $artifactName"

$sizeMiB = [math]::Round((Get-Item -LiteralPath $artifact).Length / 1MB, 2)
if ($sizeMiB -gt 30) { throw "installer is $sizeMiB MiB, above the 30 MiB release budget" }

Write-Host "Release artifact: $artifact"
Write-Host "Installer size: $sizeMiB MiB"
Write-Host "SHA-256: $hash"
