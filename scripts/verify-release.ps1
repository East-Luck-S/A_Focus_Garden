param(
  [switch]$RequireSignature,
  [switch]$RequireCleanSource,
  [switch]$PublicRelease
)

$ErrorActionPreference = "Stop"

if ($PublicRelease) {
  $RequireSignature = $true
  $RequireCleanSource = $true
}

$releaseWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$releaseManifestPath = Join-Path $releaseWorkspace "package.json"
$releaseTauriConfigPath = Join-Path $releaseWorkspace "src-tauri\tauri.conf.json"
$releaseCargoManifestPath = Join-Path $releaseWorkspace "src-tauri\Cargo.toml"
$releaseCargoLockPath = Join-Path $releaseWorkspace "src-tauri\Cargo.lock"
$releasePackageLockPath = Join-Path $releaseWorkspace "package-lock.json"
$releaseNoticePath = Join-Path $releaseWorkspace "THIRD_PARTY_NOTICES.txt"
$releaseNoticeVerifier = Join-Path $releaseWorkspace "scripts\verify-third-party-notices.ps1"
$releaseManifest = [System.IO.File]::ReadAllText($releaseManifestPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
$releaseTauriConfig = [System.IO.File]::ReadAllText($releaseTauriConfigPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
$releaseCargoManifest = [System.IO.File]::ReadAllText($releaseCargoManifestPath, [System.Text.Encoding]::UTF8)
$releaseVersion = [string]$releaseManifest.version
$releaseTarget = Join-Path $releaseWorkspace "src-tauri\target\release"
$releaseEvidencePath = Join-Path $releaseTarget "release-manifest.json"
$releaseArtifacts = @(
  Join-Path $releaseTarget "focus-garden.exe"
  Join-Path $releaseTarget "bundle\nsis\Focus Garden_${releaseVersion}_x64-setup.exe"
)

& $releaseNoticeVerifier

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

function Get-ReleaseRelativePath {
  param([string]$Path)

  $resolvedPath = (Resolve-Path -LiteralPath $Path).Path
  $workspacePrefix = $releaseWorkspace.TrimEnd("\") + "\"
  if (-not $resolvedPath.StartsWith($workspacePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Release verification failed: evidence path is outside the workspace: $resolvedPath"
  }
  return $resolvedPath.Substring($workspacePrefix.Length).Replace("\", "/")
}

if ([string]$releaseTauriConfig.productName -ne "Focus Garden" -or [string]$releaseTauriConfig.identifier -ne "com.focusgarden.desktop") {
  throw "Release verification failed: unexpected product identity in tauri.conf.json."
}
$releaseBundleTargets = @($releaseTauriConfig.bundle.targets)
if ($releaseBundleTargets.Count -ne 1 -or [string]$releaseBundleTargets[0] -ne "nsis") {
  throw "Release verification failed: the Windows release target must be exactly NSIS."
}
if ([string]$releaseTauriConfig.version -ne $releaseVersion) {
  throw "Release verification failed: package.json and tauri.conf.json versions do not match."
}

$releaseCargoPackage = [regex]::Match($releaseCargoManifest, '(?ms)^\[package\]\s*(?<body>.*?)(?=^\[|\z)')
$releaseCargoVersion = if ($releaseCargoPackage.Success) {
  $cargoVersionMatch = [regex]::Match($releaseCargoPackage.Groups['body'].Value, '(?m)^version\s*=\s*"(?<version>[^"]+)"\s*$')
  if ($cargoVersionMatch.Success) { $cargoVersionMatch.Groups['version'].Value } else { $null }
} else {
  $null
}
if ($releaseCargoVersion -ne $releaseVersion) {
  throw "Release verification failed: package.json and Cargo.toml versions do not match."
}

$releaseGitCommitOutput = @(& git -C $releaseWorkspace rev-parse HEAD)
$releaseGitExitCode = $LASTEXITCODE
if ($releaseGitExitCode -ne 0 -or $releaseGitCommitOutput.Count -eq 0) {
  throw "Release verification failed: unable to resolve the source commit."
}
$releaseGitCommit = ([string]$releaseGitCommitOutput[0]).Trim()
$releaseGitStatus = @(& git -C $releaseWorkspace status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0) {
  throw "Release verification failed: unable to inspect the source working tree."
}
$releaseWorkingTreeClean = $releaseGitStatus.Count -eq 0
if ($RequireCleanSource -and -not $releaseWorkingTreeClean) {
  throw "Release verification failed: a clean source working tree is required."
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

$releaseArtifactEvidence = @()
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

  $releaseArtifactEvidence += [ordered]@{
    path = Get-ReleaseRelativePath -Path $releaseArtifact
    length = [long]$releaseItem.Length
    version = [string]$releaseItem.VersionInfo.ProductVersion
    sha256 = $releaseHash
    signature = [ordered]@{
      status = [string]$releaseSignature.Status
      signerSubject = if ($releaseSignature.SignerCertificate) { [string]$releaseSignature.SignerCertificate.Subject } else { $null }
      signerThumbprint = if ($releaseSignature.SignerCertificate) { [string]$releaseSignature.SignerCertificate.Thumbprint } else { $null }
    }
  }

  [pscustomobject]@{
    Path = $releaseArtifact
    Length = $releaseItem.Length
    Version = $releaseItem.VersionInfo.ProductVersion
    SHA256 = $releaseHash
    Signature = $releaseSignature.Status
  } | Format-List
}

$releaseEvidence = [ordered]@{
  manifestVersion = 1
  product = [ordered]@{
    name = [string]$releaseTauriConfig.productName
    identifier = [string]$releaseTauriConfig.identifier
    version = $releaseVersion
    bundleTarget = "nsis-x64"
  }
  source = [ordered]@{
    gitCommit = $releaseGitCommit
    workingTreeClean = $releaseWorkingTreeClean
  }
  dependencies = [ordered]@{
    cargoLockSha256 = Get-ReleaseSha256 -Path $releaseCargoLockPath
    packageLockSha256 = Get-ReleaseSha256 -Path $releasePackageLockPath
    thirdPartyNotices = [ordered]@{
      path = Get-ReleaseRelativePath -Path $releaseNoticePath
      length = [long](Get-Item -LiteralPath $releaseNoticePath).Length
      sha256 = Get-ReleaseSha256 -Path $releaseNoticePath
    }
  }
  artifacts = @($releaseArtifactEvidence)
  verification = [ordered]@{
    artifactScan = "clean"
    signatureRequired = [bool]$RequireSignature
    cleanSourceRequired = [bool]$RequireCleanSource
  }
}

$releaseEvidenceJson = $releaseEvidence | ConvertTo-Json -Depth 8
$releaseUtf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($releaseEvidencePath, $releaseEvidenceJson + [Environment]::NewLine, $releaseUtf8NoBom)
$releaseEvidenceHash = Get-ReleaseSha256 -Path $releaseEvidencePath

[pscustomobject]@{
  ReleaseManifest = $releaseEvidencePath
  SHA256 = $releaseEvidenceHash
  SourceCommit = $releaseGitCommit
  SourceClean = $releaseWorkingTreeClean
} | Format-List

"ARTIFACT_SCAN_CLEAN"
