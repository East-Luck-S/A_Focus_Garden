$ErrorActionPreference = "Stop"

$noticeWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$noticeCargoLock = Join-Path $noticeWorkspace "src-tauri\Cargo.lock"
$noticePackageLock = Join-Path $noticeWorkspace "package-lock.json"
$noticePath = Join-Path $noticeWorkspace "THIRD_PARTY_NOTICES.txt"

function Get-NoticeSha256 {
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

if (-not (Test-Path -LiteralPath $noticePath -PathType Leaf)) {
  throw "Third-party notice verification failed: THIRD_PARTY_NOTICES.txt is missing."
}

$noticeText = Get-Content -LiteralPath $noticePath -Raw
$expectedCargoHash = Get-NoticeSha256 -Path $noticeCargoLock
$expectedPackageHash = Get-NoticeSha256 -Path $noticePackageLock

if (-not $noticeText.Contains("Cargo.lock SHA-256: $expectedCargoHash")) {
  throw "Third-party notice verification failed: Cargo.lock has changed. Run npm run licenses:generate."
}
if (-not $noticeText.Contains("package-lock.json SHA-256: $expectedPackageHash")) {
  throw "Third-party notice verification failed: package-lock.json has changed. Run npm run licenses:generate."
}
if (-not $noticeText.Contains("JAVASCRIPT DEPENDENCIES") -or -not $noticeText.Contains("RUST DEPENDENCIES")) {
  throw "Third-party notice verification failed: a dependency section is missing."
}
if ((Get-Item -LiteralPath $noticePath).Length -lt 10000) {
  throw "Third-party notice verification failed: generated output is unexpectedly small."
}

"THIRD_PARTY_NOTICES_OK"
