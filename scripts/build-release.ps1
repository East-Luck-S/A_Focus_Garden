param(
  [switch]$PublicRelease,
  [string]$SigningConfig
)

$ErrorActionPreference = "Stop"

$releaseWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$releaseCargoHome = if ($env:CARGO_HOME) {
  (Resolve-Path -LiteralPath $env:CARGO_HOME).Path
} else {
  Join-Path $env:USERPROFILE ".cargo"
}
$releaseTauri = Join-Path $releaseWorkspace "node_modules\.bin\tauri.cmd"
$releaseVerifier = Join-Path $PSScriptRoot "verify-release.ps1"
$releaseSeparator = [char]0x1f
$releaseRemaps = @(
  "--remap-path-prefix=$releaseWorkspace=."
  "--remap-path-prefix=$releaseCargoHome=/cargo"
) -join $releaseSeparator
$releasePreviousFlags = $env:CARGO_ENCODED_RUSTFLAGS
$releaseTauriArguments = @("build")

if ($PublicRelease -and -not $SigningConfig) {
  throw "Public release build failed: -SigningConfig is required."
}
if ($SigningConfig -and -not $PublicRelease) {
  throw "Release build failed: -SigningConfig may only be used with -PublicRelease."
}

if ($PublicRelease) {
  $releaseProductLicenseCandidates = @(
    "LICENSE"
    "LICENSE.txt"
    "EULA.txt"
  )
  $releaseProductLicenseRelativePaths = @(
    $releaseProductLicenseCandidates | Where-Object {
      Test-Path -LiteralPath (Join-Path $releaseWorkspace $_) -PathType Leaf
    }
  )
  if ($releaseProductLicenseRelativePaths.Count -ne 1) {
    throw "Public release build failed: exactly one reviewed product license is required at LICENSE, LICENSE.txt, or EULA.txt."
  }
  $releaseProductLicenseRelativePath = $releaseProductLicenseRelativePaths[0]
  $releaseProductLicensePath = (Resolve-Path -LiteralPath (Join-Path $releaseWorkspace $releaseProductLicenseRelativePath)).Path
  $releaseTrackedProductLicense = @(& git -C $releaseWorkspace ls-files -- $releaseProductLicenseRelativePath)
  if ($LASTEXITCODE -ne 0) {
    throw "Public release build failed: unable to inspect product license tracking status."
  }
  if ($releaseTrackedProductLicense.Count -ne 1) {
    throw "Public release build failed: the reviewed product license must be tracked by Git."
  }

  $releaseProductionConfigPath = Join-Path $releaseWorkspace "src-tauri\tauri.conf.json"
  $releaseProductionConfig = [System.IO.File]::ReadAllText($releaseProductionConfigPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
  $releaseConfiguredLicenseFile = [string]$releaseProductionConfig.bundle.licenseFile
  if (-not $releaseConfiguredLicenseFile) {
    throw "Public release build failed: bundle.licenseFile must include the reviewed product license in the installer."
  }
  $releaseConfiguredLicensePath = [System.IO.Path]::GetFullPath(
    (Join-Path (Split-Path -Parent $releaseProductionConfigPath) $releaseConfiguredLicenseFile)
  )
  if (-not $releaseConfiguredLicensePath.Equals($releaseProductLicensePath, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Public release build failed: bundle.licenseFile must point to the single reviewed product license."
  }

  if (-not (Test-Path -LiteralPath $SigningConfig -PathType Leaf)) {
    throw "Public release build failed: signing config is missing. Copy the example to src-tauri/tauri.signing.local.conf.json."
  }
  $releaseSigningConfig = (Resolve-Path -LiteralPath $SigningConfig).Path
  $releaseWorkspacePrefix = $releaseWorkspace.TrimEnd("\") + "\"
  if (-not $releaseSigningConfig.StartsWith($releaseWorkspacePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Public release build failed: signing config must be inside the workspace."
  }
  $releaseSigningRelativePath = $releaseSigningConfig.Substring($releaseWorkspacePrefix.Length).Replace("\", "/")
  $releaseTrackedSigningConfig = @(& git -C $releaseWorkspace ls-files -- $releaseSigningRelativePath)
  if ($LASTEXITCODE -ne 0) {
    throw "Public release build failed: unable to inspect signing config tracking status."
  }
  if ($releaseTrackedSigningConfig.Count -gt 0) {
    throw "Public release build failed: signing config must be ignored and must not be tracked by Git."
  }

  $releaseSigningText = [System.IO.File]::ReadAllText($releaseSigningConfig, [System.Text.Encoding]::UTF8)
  if ($releaseSigningText -match '(?i)password|private.?key|client.?secret|pfx') {
    throw "Public release build failed: signing config must not contain certificate secrets or private-key material."
  }
  $releaseSigningDocument = $releaseSigningText | ConvertFrom-Json
  $releaseWindowsSigning = $releaseSigningDocument.bundle.windows
  $releaseCertificateThumbprint = ([string]$releaseWindowsSigning.certificateThumbprint).Replace(" ", "").ToUpperInvariant()
  if ($releaseCertificateThumbprint -notmatch '^[0-9A-F]{40}$') {
    throw "Public release build failed: certificateThumbprint must contain exactly 40 hexadecimal characters."
  }
  if ([string]$releaseWindowsSigning.digestAlgorithm -ne "sha256") {
    throw "Public release build failed: digestAlgorithm must be sha256."
  }
  if ([string]$releaseWindowsSigning.timestampUrl -notmatch '^https?://') {
    throw "Public release build failed: timestampUrl must be an HTTP or HTTPS URL supplied by the certificate authority."
  }

  $releaseCertificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$releaseCertificateThumbprint" -ErrorAction SilentlyContinue
  if (-not $releaseCertificate -or -not $releaseCertificate.HasPrivateKey) {
    throw "Public release build failed: the configured CurrentUser certificate with a private key is unavailable."
  }
  if ($releaseCertificate.NotAfter -le (Get-Date)) {
    throw "Public release build failed: the configured code-signing certificate has expired."
  }

  $releaseTauriArguments += @("--config", $releaseSigningConfig)
}

try {
  $env:CARGO_ENCODED_RUSTFLAGS = if ($releasePreviousFlags) {
    "$releasePreviousFlags$releaseSeparator$releaseRemaps"
  } else {
    $releaseRemaps
  }

  & $releaseTauri @releaseTauriArguments
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  if ($PublicRelease) {
    & $releaseVerifier -PublicRelease
  } else {
    & $releaseVerifier
  }
} finally {
  $env:CARGO_ENCODED_RUSTFLAGS = $releasePreviousFlags
}
