param(
  [string]$ExpectedVersion
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "forgeos-mcp\Cargo.toml"
$rootPackage = Get-Content -Raw -LiteralPath (Join-Path $root "package.json") | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace($ExpectedVersion)) {
  $ExpectedVersion = [string]$rootPackage.version
}

function Read-CargoVersion([string]$Path) {
  $match = [regex]::Match((Get-Content -Raw -LiteralPath $Path), '(?m)^version\s*=\s*"([^"]+)"')
  if (-not $match.Success) { throw "version not found in $Path" }
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

$mcpManifestVersion = Read-CargoVersion $manifest
if ($mcpManifestVersion -ne $ExpectedVersion) {
  throw "forgeos-mcp manifest version $mcpManifestVersion does not match release version $ExpectedVersion"
}

$hostLine = rustc -vV | Where-Object { $_ -like "host:*" }
$hostTriple = ($hostLine -replace "^host:\s*", "").Trim()
if ($hostTriple -ne "x86_64-pc-windows-msvc") {
  throw "ForgeOS only packages x86_64-pc-windows-msvc; current host is $hostTriple"
}

Write-Host "Building forgeos-mcp release sidecar..."
$previousTauriConfig = $env:TAURI_CONFIG
try {
  # forgeos-mcp reuses the ForgeOS core crate. Disable externalBin validation
  # while bootstrapping the sidecar itself to avoid requiring yesterday's binary.
  $env:TAURI_CONFIG = '{"bundle":{"externalBin":[]}}'
  cargo build --release --manifest-path $manifest
  if ($LASTEXITCODE -ne 0) { throw "forgeos-mcp release build failed" }
} finally {
  if ($null -eq $previousTauriConfig) {
    Remove-Item Env:\TAURI_CONFIG -ErrorAction SilentlyContinue
  } else {
    $env:TAURI_CONFIG = $previousTauriConfig
  }
}

$metadata = cargo metadata --manifest-path $manifest --no-deps --format-version 1 | ConvertFrom-Json
$source = Join-Path $metadata.target_directory "release\forgeos-mcp.exe"
if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
  throw "forgeos-mcp release executable was not produced at $source"
}

$destinationDirectory = Join-Path $root "src-tauri\binaries"
New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
$destination = Join-Path $destinationDirectory "forgeos-mcp-x86_64-pc-windows-msvc.exe"
Copy-Item -LiteralPath $source -Destination $destination -Force

$version = & $destination --version
if ($LASTEXITCODE -ne 0 -or $version.Trim() -ne "forgeos-mcp $ExpectedVersion") {
  throw "packaged sidecar version check failed: $version"
}

$hash = Get-Sha256 $destination
Write-Host "Sidecar ready: $destination"
Write-Host "Sidecar SHA-256: $hash"
