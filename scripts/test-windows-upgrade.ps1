param(
  [string]$PreviousInstallerPath,
  [string]$CurrentInstallerPath,
  [string]$DatabaseSourcePath
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
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

$previousVersion = "0.1.0"
$currentVersion = [string](Get-Content -Raw -LiteralPath (Join-Path $root "package.json") | ConvertFrom-Json).version

if ([string]::IsNullOrWhiteSpace($PreviousInstallerPath)) {
  $PreviousInstallerPath = Join-Path $root "src-tauri\target\release-fixtures\v0.1.0\ForgeOS-0.1.0-windows-x64-setup.exe"
}
if ([string]::IsNullOrWhiteSpace($CurrentInstallerPath)) {
  $CurrentInstallerPath = Join-Path $root "dist-release\ForgeOS-$currentVersion-windows-x64-setup.exe"
}
if ([string]::IsNullOrWhiteSpace($DatabaseSourcePath)) {
  $DatabaseSourcePath = Join-Path $env:APPDATA "com.forgeos.desktop\database\forgeos.db"
}

$PreviousInstallerPath = [IO.Path]::GetFullPath($PreviousInstallerPath)
$CurrentInstallerPath = [IO.Path]::GetFullPath($CurrentInstallerPath)
$DatabaseSourcePath = [IO.Path]::GetFullPath($DatabaseSourcePath)
foreach ($installer in @($PreviousInstallerPath, $CurrentInstallerPath)) {
  if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "installer not found: $installer"
  }
}

function Assert-InstallerChecksum([string]$Path) {
  $checksumPath = Join-Path (Split-Path -Parent $Path) "SHA256SUMS.txt"
  if (-not (Test-Path -LiteralPath $checksumPath -PathType Leaf)) {
    throw "checksum file not found beside installer: $checksumPath"
  }
  $fileName = Split-Path -Leaf $Path
  $checksumLine = Get-Content -LiteralPath $checksumPath |
    Where-Object { $_ -match "\s+$([regex]::Escape($fileName))$" } |
    Select-Object -First 1
  if ([string]::IsNullOrWhiteSpace($checksumLine)) {
    throw "checksum entry for $fileName was not found"
  }
  $expectedHash = ($checksumLine -split "\s+")[0].Trim().ToLowerInvariant()
  $actualHash = Get-Sha256 $Path
  if ($actualHash -ne $expectedHash) {
    throw "installer SHA-256 does not match the checksum file: $fileName"
  }
}

Assert-InstallerChecksum $PreviousInstallerPath
Assert-InstallerChecksum $CurrentInstallerPath

$runningForgeOs = @(Get-Process -Name "forgeos" -ErrorAction SilentlyContinue)
if ($runningForgeOs.Count -gt 0) {
  throw "ForgeOS is running. Close the desktop app before copying the real database for upgrade acceptance."
}

$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ForgeOS"
if (Test-Path -LiteralPath $uninstallKey) {
  throw "an existing ForgeOS installation is registered; upgrade test stopped to avoid replacing it"
}

$smokeRoot = [IO.Path]::GetFullPath((Join-Path $root "src-tauri\target\installer-upgrade-smoke"))
$targetRoot = [IO.Path]::GetFullPath((Join-Path $root "src-tauri\target"))
if (-not $smokeRoot.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "upgrade test directory escaped the Tauri target directory"
}
if (Test-Path -LiteralPath $smokeRoot) {
  Remove-Item -LiteralPath $smokeRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $smokeRoot | Out-Null

$installDirectory = Join-Path $smokeRoot "ForgeOS"
$codexHome = Join-Path $smokeRoot "codex-home"
$dataDirectory = Join-Path $smokeRoot "data"
New-Item -ItemType Directory -Path $codexHome | Out-Null
New-Item -ItemType Directory -Path $dataDirectory | Out-Null
$previousCodexHome = $env:CODEX_HOME
$previousDataDirectory = $env:FORGEOS_DATA_DIR

function Invoke-Installer([string]$Path) {
  $process = Start-Process -FilePath $Path -ArgumentList @("/S", "/D=$installDirectory") -Wait -PassThru -WindowStyle Hidden
  if ($process.ExitCode -ne 0) { throw "installer failed with exit code $($process.ExitCode)" }
}

function Resolve-CodexExecutable {
  $paths = @(where.exe codex 2>$null)
  foreach ($path in $paths) {
    if ($path.EndsWith(".cmd", [StringComparison]::OrdinalIgnoreCase)) {
      $prefix = Split-Path -Parent $path
      $native = Join-Path $prefix "node_modules\@openai\codex\node_modules\@openai\codex-win32-x64\vendor\x86_64-pc-windows-msvc\bin\codex.exe"
      if (Test-Path -LiteralPath $native -PathType Leaf) { return $native }
    }
    if ($path.EndsWith(".exe", [StringComparison]::OrdinalIgnoreCase)) {
      try {
        & $path --version *> $null
        if ($LASTEXITCODE -eq 0) { return $path }
      } catch { }
    }
  }
  throw "no usable Codex CLI executable was found"
}

function Invoke-CodexAllowFailure([string[]]$Arguments) {
  $previousErrorAction = $ErrorActionPreference
  try {
    $ErrorActionPreference = "Continue"
    & $codex @Arguments *> $null
    return $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousErrorAction
  }
}

function Copy-DatabaseSnapshot {
  if (-not (Test-Path -LiteralPath $DatabaseSourcePath -PathType Leaf)) {
    Write-Host "Database upgrade check skipped because the source database was not found."
    return $null
  }
  $destination = Join-Path $dataDirectory "forgeos.db"
  Copy-Item -LiteralPath $DatabaseSourcePath -Destination $destination -Force
  foreach ($suffix in @("-wal", "-shm")) {
    $companion = "$DatabaseSourcePath$suffix"
    if (Test-Path -LiteralPath $companion -PathType Leaf) {
      Copy-Item -LiteralPath $companion -Destination "$destination$suffix" -Force
    }
  }
  return $destination
}

$codex = $null
$launchedApp = $null
try {
  $env:CODEX_HOME = $codexHome
  $env:FORGEOS_DATA_DIR = $dataDirectory
  $codex = Resolve-CodexExecutable

  Write-Host "Installing ForgeOS v$previousVersion into the isolated upgrade directory..."
  Invoke-Installer $PreviousInstallerPath

  $app = Join-Path $installDirectory "forgeos.exe"
  $mcp = Join-Path $installDirectory "forgeos-mcp.exe"
  $uninstaller = Join-Path $installDirectory "uninstall.exe"
  foreach ($file in @($app, $mcp, $uninstaller)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "installed file is missing: $file" }
  }
  $oldVersion = & $mcp --version
  if ($LASTEXITCODE -ne 0 -or $oldVersion.Trim() -ne "forgeos-mcp $previousVersion") {
    throw "v$previousVersion Sidecar version diagnostic failed: $oldVersion"
  }

  & $codex mcp add forgeos -- $mcp
  if ($LASTEXITCODE -ne 0) { throw "isolated v$previousVersion Codex MCP registration failed" }

  Write-Host "Upgrading the isolated installation to ForgeOS v$currentVersion..."
  Invoke-Installer $CurrentInstallerPath
  $newVersion = & $mcp --version
  if ($LASTEXITCODE -ne 0 -or $newVersion.Trim() -ne "forgeos-mcp $currentVersion") {
    throw "v$currentVersion Sidecar version diagnostic failed: $newVersion"
  }
  $registration = (& $codex mcp get forgeos --json | ConvertFrom-Json)
  if ([IO.Path]::GetFullPath($registration.transport.command) -ne [IO.Path]::GetFullPath($mcp)) {
    throw "the v0.1.0 to v$currentVersion upgrade did not preserve the owned Codex MCP registration"
  }

  $databaseCopy = Copy-DatabaseSnapshot
  if ($null -ne $databaseCopy) {
    $databaseCheck = & $mcp --check-database --database $databaseCopy
    if ($LASTEXITCODE -ne 0 -or $databaseCheck.Trim() -ne "database ok") {
      throw "isolated database integrity diagnostic failed before app startup: $databaseCheck"
    }
    $beforeVersion = & $mcp --database-version --database $databaseCopy
    if ($LASTEXITCODE -ne 0) { throw "unable to read the isolated database migration version" }

    $launchedApp = Start-Process -FilePath $app -PassThru -WindowStyle Hidden
    Start-Sleep -Seconds 5
    if ($launchedApp.HasExited) {
      throw "installed ForgeOS exited before the isolated database acceptance completed"
    }
    Stop-Process -Id $launchedApp.Id -Force
    $launchedApp.WaitForExit()
    $launchedApp = $null

    $databaseCheckAfter = & $mcp --check-database --database $databaseCopy
    $afterVersion = & $mcp --database-version --database $databaseCopy
    if ($LASTEXITCODE -ne 0 -or $databaseCheckAfter.Trim() -ne "database ok") {
      throw "isolated database integrity diagnostic failed after app startup: $databaseCheckAfter"
    }
    if ([int]$afterVersion -lt 21) {
      throw "installed ForgeOS did not provide Migration 21; database version is $afterVersion"
    }
    Write-Host "Isolated database accepted: migration $beforeVersion -> $afterVersion, integrity ok."
  }

  Write-Host "Testing normal uninstall after the real upgrade..."
  $uninstallProcess = Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait -PassThru -WindowStyle Hidden
  if ($uninstallProcess.ExitCode -ne 0) { throw "uninstaller failed with exit code $($uninstallProcess.ExitCode)" }
  $getAfterUninstall = Invoke-CodexAllowFailure @("mcp", "get", "forgeos", "--json")
  if ($getAfterUninstall -eq 0) { throw "normal uninstall did not remove the owned Codex MCP registration" }
  if (Test-Path -LiteralPath $uninstallKey) { throw "normal uninstall left the ForgeOS uninstall registration behind" }

  Write-Host "ForgeOS v0.1.0 to v$currentVersion upgrade acceptance passed."
} finally {
  if ($null -ne $launchedApp -and -not $launchedApp.HasExited) {
    Stop-Process -Id $launchedApp.Id -Force -ErrorAction SilentlyContinue
  }
  if ($null -ne $codex) {
    Invoke-CodexAllowFailure @("mcp", "remove", "forgeos") | Out-Null
  }
  if (Test-Path -LiteralPath (Join-Path $installDirectory "uninstall.exe")) {
    Start-Process -FilePath (Join-Path $installDirectory "uninstall.exe") -ArgumentList "/S" -Wait -WindowStyle Hidden
  }
  if ($null -eq $previousCodexHome) {
    Remove-Item Env:\CODEX_HOME -ErrorAction SilentlyContinue
  } else {
    $env:CODEX_HOME = $previousCodexHome
  }
  if ($null -eq $previousDataDirectory) {
    Remove-Item Env:\FORGEOS_DATA_DIR -ErrorAction SilentlyContinue
  } else {
    $env:FORGEOS_DATA_DIR = $previousDataDirectory
  }
  if (Test-Path -LiteralPath $smokeRoot) {
    Remove-Item -LiteralPath $smokeRoot -Recurse -Force
  }
}
