$ErrorActionPreference = "Stop"

$noticeWorkspace = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$noticeCargoManifest = Join-Path $noticeWorkspace "src-tauri\Cargo.toml"
$noticeCargoConfig = Join-Path $noticeWorkspace "src-tauri\about.toml"
$noticeCargoLock = Join-Path $noticeWorkspace "src-tauri\Cargo.lock"
$noticePackageLock = Join-Path $noticeWorkspace "package-lock.json"
$noticeTemplate = Join-Path $noticeWorkspace "scripts\licenses\rust-notices.hbs"
$noticeOutput = Join-Path $noticeWorkspace "THIRD_PARTY_NOTICES.txt"
$noticeGenerationDirectory = Join-Path $noticeWorkspace "src-tauri\target\license-generation"
$noticeRustOutput = Join-Path $noticeGenerationDirectory "rust-notices.txt"

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

if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
  throw "cargo-about is required. Install it with: cargo install --locked cargo-about --features cli"
}

[System.IO.Directory]::CreateDirectory($noticeGenerationDirectory) | Out-Null

& cargo about generate `
  --manifest-path $noticeCargoManifest `
  --config $noticeCargoConfig `
  --locked `
  --fail `
  --output-file $noticeRustOutput `
  $noticeTemplate
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

$nodeInventoryScript = @'
const fs = require('fs');
const lockPath = process.argv[1];
const lock = JSON.parse(fs.readFileSync(lockPath, 'utf8'));
const packages = Object.entries(lock.packages)
  .filter(([path, metadata]) => path && metadata.dev !== true)
  .map(([path, metadata]) => ({
    path,
    name: path.replace(/^node_modules\//, ''),
    version: metadata.version,
    license: metadata.license,
  }))
  .sort((left, right) => left.name.localeCompare(right.name));
process.stdout.write(JSON.stringify(packages));
'@
$nodeInventoryJson = & node -e $nodeInventoryScript $noticePackageLock
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}
$nodePackages = $nodeInventoryJson | ConvertFrom-Json

$noticeParts = New-Object System.Collections.Generic.List[string]
$noticeParts.Add("FOCUS GARDEN THIRD-PARTY NOTICES")
$noticeParts.Add("================================")
$noticeParts.Add("")
$noticeParts.Add("This file covers third-party software distributed with Focus Garden.")
$noticeParts.Add("It does not grant a license to Focus Garden itself.")
$noticeParts.Add("")
$noticeParts.Add("Cargo.lock SHA-256: $(Get-NoticeSha256 -Path $noticeCargoLock)")
$noticeParts.Add("package-lock.json SHA-256: $(Get-NoticeSha256 -Path $noticePackageLock)")
$noticeParts.Add("Generator: $((& cargo about --version) -join ' ')")
$noticeParts.Add("")
$noticeParts.Add("JAVASCRIPT DEPENDENCIES")
$noticeParts.Add("=======================")

foreach ($nodePackage in $nodePackages) {
  $packageDirectory = Join-Path $noticeWorkspace ($nodePackage.path.Replace("/", "\"))
  $licenseFiles = @(Get-ChildItem -LiteralPath $packageDirectory -File | Where-Object { $_.Name -match "^(LICENSE|COPYING|NOTICE)" } | Sort-Object Name)
  if ($licenseFiles.Count -eq 0) {
    throw "No license or notice file found for npm runtime package $($nodePackage.name) $($nodePackage.version)."
  }

  $noticeParts.Add("")
  $noticeParts.Add("--------------------------------------------------------------------------------")
  $noticeParts.Add("$($nodePackage.name) $($nodePackage.version)")
  $noticeParts.Add("Declared license: $($nodePackage.license)")
  foreach ($licenseFile in $licenseFiles) {
    $noticeParts.Add("")
    $noticeParts.Add("[$($licenseFile.Name)]")
    $noticeParts.Add((Get-Content -LiteralPath $licenseFile.FullName -Raw).TrimEnd())
  }
}

$noticeParts.Add("")
$noticeParts.Add("================================================================================")
$noticeParts.Add("")
$noticeParts.Add((Get-Content -LiteralPath $noticeRustOutput -Raw).TrimEnd())
$noticeParts.Add("")

$noticeText = $noticeParts -join [Environment]::NewLine
$noticeText = [System.Text.RegularExpressions.Regex]::Replace(
  $noticeText,
  "[ \t]+(?=\r?$)",
  "",
  [System.Text.RegularExpressions.RegexOptions]::Multiline
)
$noticeEncoding = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($noticeOutput, $noticeText, $noticeEncoding)

"Generated $noticeOutput"
