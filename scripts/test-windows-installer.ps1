param(
  [string]$InstallerPath
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
  $InstallerPath = Join-Path $root "dist-release\ForgeOS-0.1.0-windows-x64-setup.exe"
}
$InstallerPath = [IO.Path]::GetFullPath($InstallerPath)
if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
  throw "installer not found: $InstallerPath"
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
  $expectedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $expectedMcp).Hash
  $installedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $mcp).Hash
  if ($expectedHash -ne $installedHash) { throw "installed Sidecar does not match the release build" }

  $version = & $mcp --version
  if ($LASTEXITCODE -ne 0 -or $version.Trim() -ne "forgeos-mcp 0.1.0") {
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
