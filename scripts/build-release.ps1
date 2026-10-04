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

try {
  $env:CARGO_ENCODED_RUSTFLAGS = if ($releasePreviousFlags) {
    "$releasePreviousFlags$releaseSeparator$releaseRemaps"
  } else {
    $releaseRemaps
  }

  & $releaseTauri build
  if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
  }

  & $releaseVerifier
} finally {
  $env:CARGO_ENCODED_RUSTFLAGS = $releasePreviousFlags
}
