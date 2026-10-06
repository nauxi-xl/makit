# Builds Makit, then has that Makit build its own Source tree (dogfooding; bash twin: ./bootstrap).
#
#   stage0  Cargo builds makit.
#   stage1  stage0 builds this Source tree into $env:OUT\stage1, using .makit/config.toml.
#
# Until the executor lands (#8), stage1 ends at "not implemented yet". Getting that far still
# proves stage0 accepts this Source tree: Project manifest, Output tree and host Toolchain.
$ErrorActionPreference = 'Stop'

# OUT is relative to where bootstrap was started, like any other path the caller passes.
$Out = if ($env:OUT) { $env:OUT } else { 'out/bootstrap' }
if (-not [IO.Path]::IsPathRooted($Out)) { $Out = Join-Path (Get-Location) $Out }
Set-Location $PSScriptRoot

function Fail($msg) { Write-Host "[bootstrap] $msg" -ForegroundColor Red; exit 1 }
function Info($msg) { Write-Host "[bootstrap] $msg" }

Info 'stage0: cargo build'
& cargo build --locked --quiet
if ($LASTEXITCODE -ne 0) { Fail "cargo build failed ($LASTEXITCODE)" }
# Windows PowerShell 5.1 has no $IsWindows, and only runs on Windows.
$Exe = if ($PSVersionTable.PSEdition -eq 'Desktop' -or $IsWindows) { 'makit.exe' } else { 'makit' }
$Makit = Join-Path 'target/debug' $Exe

# Forget the host Toolchain an earlier run recorded, so a changed host never gets in the way.
# Only that file: makit itself decides whether $Out/stage1 is an acceptable Output tree.
$Stage1 = Join-Path $Out 'stage1'
Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $Stage1 '.host-toolchain.toml')
Info "stage1: $Makit -O $Stage1 build"
# makit reports through stderr and exit codes; 'Stop' would turn stderr into a terminating error.
$ErrorActionPreference = 'Continue'
$output = & $Makit -O $Stage1 build 2>&1 | ForEach-Object { "$_" }
$status = $LASTEXITCODE
$ErrorActionPreference = 'Stop'
$output | ForEach-Object { Write-Host $_ }

if ($status -eq 0) {
  Info 'stage1 built'
} elseif ($status -eq 1 -and ($output -match 'is not implemented yet')) {
  Info 'stage1 accepted the Source tree; building it waits for the executor (#8)'
} else {
  Fail "stage1 failed (exit $status)"
}
# A tolerated stage1 still leaves $LASTEXITCODE at 1, which would become the script's exit code.
exit 0
