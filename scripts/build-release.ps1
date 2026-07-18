$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Read-CargoVersion([string]$Path) {
  $match = [regex]::Match((Get-Content -Raw -LiteralPath $Path), '(?m)^version\s*=\s*"([^"]+)"')
  if (-not $match.Success) { throw "version not found in $Path" }
  return $match.Groups[1].Value
}

$packageVersion = (Get-Content -Raw -LiteralPath "package.json" | ConvertFrom-Json).version
$tauriVersion = (Get-Content -Raw -LiteralPath "src-tauri\tauri.conf.json" | ConvertFrom-Json).version
$desktopVersion = Read-CargoVersion "src-tauri\Cargo.toml"
$mcpVersion = Read-CargoVersion "forgeos-mcp\Cargo.toml"
$versions = @(@($packageVersion, $tauriVersion, $desktopVersion, $mcpVersion) | Select-Object -Unique)
if ($versions.Count -ne 1 -or $versions[0] -ne "0.1.0") {
  throw "release versions must all equal 0.1.0; found: $($versions -join ', ')"
}

Write-Host "Building ForgeOS v$packageVersion Windows installer..."
npm run tauri:build
if ($LASTEXITCODE -ne 0) { throw "Tauri release build failed" }

$sidecarSource = Join-Path $root "src-tauri\binaries\forgeos-mcp-x86_64-pc-windows-msvc.exe"
$sidecarStaged = Join-Path $root "src-tauri\target\release\forgeos-mcp.exe"
if (-not (Test-Path -LiteralPath $sidecarSource -PathType Leaf) -or
    -not (Test-Path -LiteralPath $sidecarStaged -PathType Leaf)) {
  throw "Tauri did not stage the forgeos-mcp sidecar"
}
$sourceSidecarHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $sidecarSource).Hash
$stagedSidecarHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $sidecarStaged).Hash
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
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifact).Hash.ToLowerInvariant()
Set-Content -LiteralPath (Join-Path $outputDirectory "SHA256SUMS.txt") -Encoding ascii -Value "$hash  $artifactName"

$sizeMiB = [math]::Round((Get-Item -LiteralPath $artifact).Length / 1MB, 2)
if ($sizeMiB -gt 30) { throw "installer is $sizeMiB MiB, above the 30 MiB release budget" }

Write-Host "Release artifact: $artifact"
Write-Host "Installer size: $sizeMiB MiB"
Write-Host "SHA-256: $hash"
