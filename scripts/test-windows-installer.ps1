param(
  [string]$InstallerPath,
  [string]$ExpectedVersion
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

if ([string]::IsNullOrWhiteSpace($ExpectedVersion)) {
  $ExpectedVersion = [string](Get-Content -Raw -LiteralPath (Join-Path $root "package.json") | ConvertFrom-Json).version
}
if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
  $InstallerPath = Join-Path $root "dist-release\ForgeOS-$ExpectedVersion-windows-x64-setup.exe"
}
$InstallerPath = [IO.Path]::GetFullPath($InstallerPath)
if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw "installer not found: $InstallerPath"
}

$checksumPath = Join-Path (Split-Path -Parent $InstallerPath) "SHA256SUMS.txt"
if (-not (Test-Path -LiteralPath $checksumPath -PathType Leaf)) {
  throw "checksum file not found: $checksumPath"
}
$expectedInstallerHash = ((Get-Content -LiteralPath $checksumPath -TotalCount 1) -split "\s+")[0].Trim()
$actualInstallerHash = Get-Sha256 $InstallerPath
if ($actualInstallerHash -ne $expectedInstallerHash.ToLowerInvariant()) {
  throw "installer SHA-256 does not match SHA256SUMS.txt"
}

$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\ForgeOS"
if (Test-Path -LiteralPath $uninstallKey) {
  throw "an existing ForgeOS installation is registered; installer smoke test stopped to avoid replacing it"
}

$smokeRoot = [IO.Path]::GetFullPath((Join-Path $root "src-tauri\target\installer-smoke"))
$targetRoot = [IO.Path]::GetFullPath((Join-Path $root "src-tauri\target"))
if (-not $smokeRoot.StartsWith($targetRoot, [StringComparison]::OrdinalIgnoreCase)) {
  throw "smoke test directory escaped the Tauri target directory"
}
if (Test-Path -LiteralPath $smokeRoot) {
  Remove-Item -LiteralPath $smokeRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $smokeRoot | Out-Null

$installDirectory = Join-Path $smokeRoot "ForgeOS"
$codexHome = Join-Path $smokeRoot "codex-home"
New-Item -ItemType Directory -Path $codexHome | Out-Null
$previousCodexHome = $env:CODEX_HOME

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

$codex = $null
try {
  $env:CODEX_HOME = $codexHome
  $codex = Resolve-CodexExecutable

  Write-Host "Installing into isolated smoke directory..."
  Invoke-Installer $InstallerPath

  $app = Join-Path $installDirectory "forgeos.exe"
  $mcp = Join-Path $installDirectory "forgeos-mcp.exe"
  $uninstaller = Join-Path $installDirectory "uninstall.exe"
  foreach ($file in @($app, $mcp, $uninstaller)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "installed file is missing: $file" }
  }

  $expectedMcp = Join-Path $root "src-tauri\binaries\forgeos-mcp-x86_64-pc-windows-msvc.exe"
  $expectedHash = Get-Sha256 $expectedMcp
  $installedHash = Get-Sha256 $mcp
  if ($expectedHash -ne $installedHash) { throw "installed Sidecar does not match the release build" }

  $version = & $mcp --version
  if ($LASTEXITCODE -ne 0 -or $version.Trim() -ne "forgeos-mcp $ExpectedVersion") {
    throw "installed Sidecar version diagnostic failed: $version"
  }

  $database = Join-Path $env:APPDATA "com.forgeos.desktop\database\forgeos.db"
  if (Test-Path -LiteralPath $database -PathType Leaf) {
    $databaseCheck = & $mcp --check-database --database $database
    if ($LASTEXITCODE -ne 0 -or $databaseCheck.Trim() -ne "database ok") {
      throw "installed Sidecar database diagnostic failed: $databaseCheck"
    }
  } else {
    Write-Host "Database diagnostic skipped because no existing ForgeOS database was found."
  }

  & $codex mcp add forgeos -- $mcp
  if ($LASTEXITCODE -ne 0) { throw "isolated Codex MCP registration failed" }
  $registration = (& $codex mcp get forgeos --json | ConvertFrom-Json)
  if ([IO.Path]::GetFullPath($registration.transport.command) -ne [IO.Path]::GetFullPath($mcp)) {
    throw "isolated Codex MCP registration points to an unexpected command"
  }

  Write-Host "Testing same-version reinstall preservation..."
  Invoke-Installer $InstallerPath
  $registrationAfterReinstall = (& $codex mcp get forgeos --json | ConvertFrom-Json)
  if ([IO.Path]::GetFullPath($registrationAfterReinstall.transport.command) -ne [IO.Path]::GetFullPath($mcp)) {
    throw "same-version reinstall did not preserve the owned Codex MCP registration"
  }

  # Force the existing install to look older so passive NSIS follows the real
  # upgrade-uninstall path. The custom marker must preserve the MCP entry.
  Set-ItemProperty -LiteralPath $uninstallKey -Name DisplayVersion -Value "0.0.9"
  Write-Host "Testing upgrade preservation..."
  Invoke-Installer $InstallerPath
  $registrationAfterUpgrade = (& $codex mcp get forgeos --json | ConvertFrom-Json)
  if ([IO.Path]::GetFullPath($registrationAfterUpgrade.transport.command) -ne [IO.Path]::GetFullPath($mcp)) {
    throw "upgrade did not preserve the owned Codex MCP registration"
  }

  Write-Host "Testing normal uninstall cleanup..."
  $uninstallProcess = Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait -PassThru -WindowStyle Hidden
  if ($uninstallProcess.ExitCode -ne 0) { throw "uninstaller failed with exit code $($uninstallProcess.ExitCode)" }
  $getAfterUninstall = Invoke-CodexAllowFailure @("mcp", "get", "forgeos", "--json")
  if ($getAfterUninstall -eq 0) { throw "normal uninstall did not remove the owned Codex MCP registration" }
  if (Test-Path -LiteralPath $uninstallKey) { throw "normal uninstall left the ForgeOS uninstall registration behind" }

  Write-Host "Windows installer smoke test passed."
} finally {
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
  if (Test-Path -LiteralPath $smokeRoot) {
    Remove-Item -LiteralPath $smokeRoot -Recurse -Force
  }
}
