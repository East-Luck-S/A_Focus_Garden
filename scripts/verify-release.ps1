param(
  [switch]$RequireSignature
)

$ErrorActionPreference = "Stop"

$releaseWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$releaseManifestPath = Join-Path $releaseWorkspace "package.json"
$releaseManifest = Get-Content -LiteralPath $releaseManifestPath -Raw | ConvertFrom-Json
$releaseVersion = [string]$releaseManifest.version
$releaseTarget = Join-Path $releaseWorkspace "src-tauri\target\release"
$releaseArtifacts = @(
  Join-Path $releaseTarget "focus-garden.exe"
  Join-Path $releaseTarget "bundle\nsis\Focus Garden_${releaseVersion}_x64-setup.exe"
)

if (-not (Get-Command Get-AuthenticodeSignature -ErrorAction SilentlyContinue)) {
  $securityModule = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1"
  Import-Module -Name $securityModule -ErrorAction Stop
}

function Get-ReleaseSha256 {
  param([string]$Path)

  $hashAlgorithm = [System.Security.Cryptography.SHA256]::Create()
  $hashStream = [System.IO.File]::OpenRead($Path)
  try {
    $hashBytes = $hashAlgorithm.ComputeHash($hashStream)
    return [System.BitConverter]::ToString($hashBytes).Replace("-", "")
  } finally {
    $hashStream.Dispose()
    $hashAlgorithm.Dispose()
  }
}

$sourceMaps = @(Get-ChildItem -LiteralPath (Join-Path $releaseWorkspace "dist") -Recurse -File -Filter "*.map" -ErrorAction SilentlyContinue)
if ($sourceMaps.Count -gt 0) {
  throw "Release verification failed: source map files are present in dist."
}

$forbiddenLiterals = @(
  [pscustomobject]@{ Label = "absolute Windows user path"; Value = "C:\Users\" }
  [pscustomobject]@{ Label = "absolute Windows user URL path"; Value = "C:/Users/" }
  [pscustomobject]@{ Label = "build workspace path"; Value = $releaseWorkspace }
  [pscustomobject]@{ Label = "build workspace URL path"; Value = $releaseWorkspace.Replace("\", "/") }
  [pscustomobject]@{ Label = "development HTTP endpoint"; Value = "http://localhost" }
  [pscustomobject]@{ Label = "development WebSocket endpoint"; Value = "ws://localhost" }
  [pscustomobject]@{ Label = "private key marker"; Value = "BEGIN PRIVATE KEY" }
  [pscustomobject]@{ Label = "RSA private key marker"; Value = "BEGIN RSA PRIVATE KEY" }
  [pscustomobject]@{ Label = "OpenSSH private key marker"; Value = "BEGIN OPENSSH PRIVATE KEY" }
)
$secretPattern = "(?i)(?:sk-[a-z0-9_-]{20,}|ghp_[a-z0-9]{20,}|github_pat_[a-z0-9_]{20,}|npm_[a-z0-9]{20,}|AKIA[0-9A-Z]{16})"

foreach ($releaseArtifact in $releaseArtifacts) {
  if (-not (Test-Path -LiteralPath $releaseArtifact -PathType Leaf)) {
    throw "Release verification failed: missing artifact $releaseArtifact"
  }

  $releaseBytes = [System.IO.File]::ReadAllBytes($releaseArtifact)
  $releaseUtf8Text = [System.Text.Encoding]::UTF8.GetString($releaseBytes)
  $releaseUnicodeText = [System.Text.Encoding]::Unicode.GetString($releaseBytes)

  foreach ($forbidden in $forbiddenLiterals) {
    $utf8Match = $releaseUtf8Text.IndexOf($forbidden.Value, [System.StringComparison]::OrdinalIgnoreCase) -ge 0
    $unicodeMatch = $releaseUnicodeText.IndexOf($forbidden.Value, [System.StringComparison]::OrdinalIgnoreCase) -ge 0
    if ($utf8Match -or $unicodeMatch) {
      throw "Release verification failed: $($forbidden.Label) found in $releaseArtifact"
    }
  }

  if ($releaseUtf8Text -match $secretPattern -or $releaseUnicodeText -match $secretPattern) {
    throw "Release verification failed: key-like token found in $releaseArtifact"
  }

  $releaseItem = Get-Item -LiteralPath $releaseArtifact
  if ($releaseItem.VersionInfo.ProductName -ne "Focus Garden") {
    throw "Release verification failed: unexpected product name in $releaseArtifact"
  }
  if ($releaseItem.VersionInfo.FileVersion -ne $releaseVersion -or $releaseItem.VersionInfo.ProductVersion -ne $releaseVersion) {
    throw "Release verification failed: artifact version does not match $releaseVersion in $releaseArtifact"
  }

  $releaseHash = Get-ReleaseSha256 -Path $releaseArtifact
  $releaseSignature = Get-AuthenticodeSignature -LiteralPath $releaseArtifact
  if ($RequireSignature -and $releaseSignature.Status -ne "Valid") {
    throw "Release verification failed: a valid Authenticode signature is required for $releaseArtifact"
  }

  [pscustomobject]@{
    Path = $releaseArtifact
    Length = $releaseItem.Length
    Version = $releaseItem.VersionInfo.ProductVersion
    SHA256 = $releaseHash
    Signature = $releaseSignature.Status
  } | Format-List
}

"ARTIFACT_SCAN_CLEAN"
