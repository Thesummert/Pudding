$ErrorActionPreference = 'Stop'

function Fail([string]$Message) {
    throw $Message
}

function Invoke-Checked([string]$Command, [string[]]$Arguments) {
    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        Fail "$Command failed with exit code $LASTEXITCODE."
    }
}

$bundles = 'nsis,msi'
$skipInstall = $false
for ($i = 0; $i -lt $args.Count; $i++) {
    switch ($args[$i]) {
        '--help' { Write-Host @'
Usage: npm run build:windows -- [options]
       powershell -ExecutionPolicy Bypass -File scripts/build-windows.ps1 [options]

Build Windows Release installers for the current Rust host target.
  --bundles <formats>  nsis, msi, or nsis,msi (default)
  --skip-install       Use existing node_modules instead of running npm ci
  -h, --help           Show this help
'@; exit 0 }
        '-h' { Write-Host 'Usage: npm run build:windows -- [--bundles nsis,msi] [--skip-install]'; exit 0 }
        '--skip-install' { $skipInstall = $true }
        '--bundles' {
            $i++
            if ($i -ge $args.Count -or $args[$i] -notin @('nsis', 'msi', 'nsis,msi', 'msi,nsis')) {
                Fail '--bundles must be nsis, msi, nsis,msi, or msi,nsis.'
            }
            $bundles = $args[$i]
        }
        default { Fail "Unknown argument: $($args[$i]). Use --help for usage." }
    }
}

if ($env:OS -ne 'Windows_NT') { Fail 'This script must run on Windows.' }
Push-Location (Join-Path $PSScriptRoot '..')
try {
    foreach ($tool in @('node.exe', 'npm.cmd', 'cargo.exe', 'rustc.exe')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            Fail "Missing $tool. Install the Windows build prerequisites described in README.md."
        }
    }
    $nodeVersionText = & node.exe --version
    if ($LASTEXITCODE -ne 0 -or $nodeVersionText -notmatch '^v(\d+)\.(\d+)\.') {
        Fail 'Unable to determine the Node.js version.'
    }
    $major = [int]$Matches[1]
    $minor = [int]$Matches[2]
    if (-not (($major -eq 20 -and $minor -ge 19) -or ($major -eq 22 -and $minor -ge 12) -or $major -gt 22)) {
        Fail 'Node.js 20.19+ (20.x) or 22.12+ is required.'
    }
    $rustInfo = & rustc.exe -vV
    if ($LASTEXITCODE -ne 0) { Fail 'Unable to inspect the Rust toolchain.' }
    if (-not ($rustInfo -match '^host: .*-pc-windows-msvc$')) {
        Fail 'An MSVC Windows Rust toolchain is required.'
    }
    if ($skipInstall) {
        foreach ($tool in @('tauri.cmd', 'vite.cmd', 'svelte-check.cmd')) {
            if (-not (Test-Path -LiteralPath (Join-Path 'node_modules/.bin' $tool) -PathType Leaf)) {
                Fail 'Frontend dependencies are incomplete. Run npm ci or omit --skip-install.'
            }
        }
    } else {
        Invoke-Checked 'npm.cmd' @('ci')
    }
    Write-Host "Building Windows Release installers: $bundles"
    Invoke-Checked 'npm.cmd' @('run', 'tauri', '--', 'build', '--ci', '--bundles', $bundles, '--', '--locked')
    $metadata = & cargo.exe metadata --manifest-path src-tauri/Cargo.toml --no-deps --format-version 1 --locked --offline
    if ($LASTEXITCODE -ne 0) { Fail 'Unable to determine the Cargo target directory.' }
    $bundleDirectory = Join-Path ($metadata | ConvertFrom-Json).target_directory 'release/bundle'
    foreach ($format in ($bundles -split ',')) {
        $extension = if ($format -eq 'nsis') { '*.exe' } else { '*.msi' }
        $artifacts = @(Get-ChildItem -Path (Join-Path (Join-Path $bundleDirectory $format) $extension) -File -ErrorAction SilentlyContinue)
        if ($artifacts.Count -eq 0) { Fail "No $format installer found in $bundleDirectory." }
        $artifacts | ForEach-Object { Write-Host $_.FullName }
    }
} finally {
    Pop-Location
}
