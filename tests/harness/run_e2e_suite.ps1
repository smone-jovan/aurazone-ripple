<#
.SYNOPSIS
    Automated 4-Tier End-to-End Test Suite Runner for AuraZoneRipple_Rust.
    Strictly enforces non-deceptive pass semantics, automatic Scoop toolchain
    discovery, and comprehensive exit code integrity.

.PARAMETER Configuration
    Build profile to inspect ("debug" or "release"). Default is "debug".

.PARAMETER VerboseOutput
    If specified, prints detailed per-test assertions and stdout.

.PARAMETER SkipCargo
    If specified, skips cargo test and executes only direct binary smoke checks.
    NOTE: Skipping the Cargo test phase prevents a clean SUCCESS verdict and yields exit code 1.

.PARAMETER ProjectDirectory
    Optional override path to AuraZoneRipple_Rust project root. Default is autodetection
    relative to script location ($ScriptDir\..\..).
#>

param(
    [string]$Configuration = "debug",
    [switch]$VerboseOutput = $false,
    [switch]$SkipCargo = $false,
    [string]$ProjectDirectory = ""
)

$ErrorActionPreference = "Stop"

if ($ProjectDirectory -and (Test-Path $ProjectDirectory)) {
    $ProjectRoot = (Resolve-Path $ProjectDirectory).Path
} else {
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $ProjectRoot = (Resolve-Path "$ScriptDir\..\..").Path
}
Set-Location $ProjectRoot

$PassCount = 0
$FailCount = 0
$SkippedCount = 0
$MissingPrereqs = [System.Collections.Generic.List[string]]::new()
$TotalStartTime = [System.Diagnostics.Stopwatch]::StartNew()

function Report-Pass([string]$testName, [string]$detail = "") {
    $script:PassCount++
    Write-Host "  [PASS] $testName" -ForegroundColor Green
    if ($detail -and $VerboseOutput) {
        Write-Host "         $detail" -ForegroundColor DarkGray
    }
}

function Report-Fail([string]$testName, [string]$errorMsg = "") {
    $script:FailCount++
    Write-Host "  [FAIL] $testName" -ForegroundColor Red
    if ($errorMsg) {
        Write-Host "         Error: $errorMsg" -ForegroundColor DarkRed
    }
}

function Report-Skip([string]$testName, [string]$reason = "") {
    $script:SkippedCount++
    Write-Host "  [SKIP] $testName" -ForegroundColor DarkYellow
    if ($reason) {
        Write-Host "         Reason: $reason" -ForegroundColor Yellow
    }
}

function Invoke-NativeCli([string]$exePath, [string[]]$cliArgs) {
    # Run via cmd.exe /c to isolate stderr stream from PowerShell terminating NativeCommandError
    $argString = $cliArgs -join " "
    $output = cmd.exe /c "`"$exePath`" $argString 2>&1"
    return @{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join "`n")
    }
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  AuraZoneRipple_Rust: 4-Tier E2E Test Suite Runner       " -ForegroundColor Cyan
Write-Host "  Target: ASUS ROG Strix G513RC (Aura Direct Col05)       " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "Project Root : $ProjectRoot" -ForegroundColor DarkGray
Write-Host "Configuration: $Configuration" -ForegroundColor DarkGray
Write-Host "Timestamp    : $([System.DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'))" -ForegroundColor DarkGray
Write-Host ""

# =========================================================================
# Toolchain Auto-Discovery (Scoop rust-gnu/rust-msvc, GCC, Rustup, Standard)
# =========================================================================
$UserProfile = if ($env:USERPROFILE) { $env:USERPROFILE } else { [System.Environment]::GetFolderPath('UserProfile') }
$ScoopRoot = if ($env:SCOOP) { $env:SCOOP } else { "$UserProfile\scoop" }

# Candidate list for cargo.exe
$CargoCandidates = @(
    # Scoop Rust GNU (primary on host)
    "$ScoopRoot\apps\rust-gnu\current\bin\cargo.exe",
    "$ScoopRoot\apps\rust-gnu\1.100.0\bin\cargo.exe",
    # Scoop Rust MSVC (auditor reference)
    "$ScoopRoot\apps\rust-msvc\current\bin\cargo.exe",
    "$ScoopRoot\apps\rust-msvc\1.100.0\bin\cargo.exe",
    # Scoop shims
    "$ScoopRoot\shims\cargo.exe",
    # Standard rustup installation
    "$UserProfile\.cargo\bin\cargo.exe",
    # System wide install
    "C:\Program Files\Rust\bin\cargo.exe"
)

# Dynamic wildcard scan for any Scoop rust-* version
$DynamicScoopCargo = Get-ChildItem -Path "$ScoopRoot\apps\rust-*\*\bin\cargo.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
if ($DynamicScoopCargo) {
    $CargoCandidates += $DynamicScoopCargo
}

$CargoCmd = $null
# 1. Check active PATH first
$ActiveCargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($ActiveCargo) {
    $CargoCmd = $ActiveCargo.Source
} else {
    foreach ($cand in $CargoCandidates) {
        if ($cand -and (Test-Path $cand)) {
            $CargoCmd = (Resolve-Path $cand).Path
            break
        }
    }
}

# Prepend discovered cargo directory and Scoop shims to PATH
if ($CargoCmd) {
    $cargoDir = Split-Path -Parent $CargoCmd
    if ($env:PATH -notlike "*$cargoDir*") {
        $env:PATH = "$cargoDir;$env:PATH"
    }
    $shimsDir = "$ScoopRoot\shims"
    if ((Test-Path $shimsDir) -and ($env:PATH -notlike "*$shimsDir*")) {
        $env:PATH = "$shimsDir;$env:PATH"
    }
}

# 2. Check GCC / Linker (essential for rust-gnu toolchain)
$GccCandidates = @(
    "$ScoopRoot\apps\gcc\current\bin\gcc.exe",
    "$ScoopRoot\apps\gcc\15.2.0\bin\gcc.exe",
    "$ScoopRoot\shims\gcc.exe"
)
$DynamicScoopGcc = Get-ChildItem -Path "$ScoopRoot\apps\gcc\*\bin\gcc.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FullName
if ($DynamicScoopGcc) {
    $GccCandidates += $DynamicScoopGcc
}

$GccCmd = $null
$ActiveGcc = Get-Command gcc -ErrorAction SilentlyContinue
if ($ActiveGcc) {
    $GccCmd = $ActiveGcc.Source
} else {
    foreach ($g in $GccCandidates) {
        if ($g -and (Test-Path $g)) {
            $GccCmd = (Resolve-Path $g).Path
            $gccDir = Split-Path -Parent $GccCmd
            if ($env:PATH -notlike "*$gccDir*") {
                $env:PATH = "$gccDir;$env:PATH"
            }
            break
        }
    }
}

# =========================================================================
# Phase 1: Toolchain Detection & Cargo Integration Tests
# =========================================================================
Write-Host "[1/3] Cargo Integration Tests (Tiers 1 to 4)..." -ForegroundColor Yellow

if (-not $CargoCmd) {
    Report-Fail "Phase 1: Cargo Toolchain Precondition" "Cargo toolchain not found in active PATH or Scoop directories ($ScoopRoot\apps\rust-*)."
    $MissingPrereqs.Add("Cargo toolchain (cargo.exe)")
    $script:SkippedCount++
} elseif ($SkipCargo) {
    Report-Skip "Phase 1: Cargo Integration Tests" "Explicitly skipped via -SkipCargo parameter."
} else {
    Write-Host "  Using cargo: $CargoCmd" -ForegroundColor DarkGray
    if ($GccCmd) {
        Write-Host "  Using gcc linker: $GccCmd" -ForegroundColor DarkGray
    }
    $nocapture = if ($VerboseOutput) { "--nocapture" } else { "" }
    
    try {
        & $CargoCmd test --test e2e_cli_tests -- --test-threads=1 $nocapture
        if ($LASTEXITCODE -eq 0) {
            Report-Pass "4-Tier Cargo Integration Suite (e2e_cli_tests)" "All unit and integration tests passed."
        } else {
            Report-Fail "4-Tier Cargo Integration Suite (e2e_cli_tests)" "cargo test exited with code $LASTEXITCODE"
        }
    } catch {
        Report-Fail "4-Tier Cargo Integration Suite (e2e_cli_tests)" $_.Exception.Message
    }
}

# =========================================================================
# Phase 2: Direct Binary Opaque-Box Smoke Checks
# =========================================================================
Write-Host ""
Write-Host "[2/3] Standalone Binary Opaque-Box Smoke Checks..." -ForegroundColor Yellow

$BinPath = "$ProjectRoot\target\$Configuration\aurazone_ripple.exe"
if (-not (Test-Path $BinPath)) {
    if ($Configuration -eq "debug") {
        # Check alternate release path only for default debug
        $AltBinPath = "$ProjectRoot\target\release\aurazone_ripple.exe"
        if (Test-Path $AltBinPath) {
            $BinPath = $AltBinPath
        }
    }
}

if (-not (Test-Path $BinPath)) {
    Report-Fail "Phase 2: Target Binary Precondition" "Binary not found at '$BinPath'. Standalone CLI smoke checks cannot execute. Build with 'cargo build' first."
    $MissingPrereqs.Add("Target binary ($BinPath)")
    $script:SkippedCount += 5
} else {
    Write-Host "  Testing binary: $BinPath" -ForegroundColor DarkGray

    # Check 1: --help
    $resHelp = Invoke-NativeCli $BinPath @("--help")
    if ($resHelp.ExitCode -eq 0 -and ($resHelp.Output -match "Usage:" -or $resHelp.Output -match "aurazone_ripple")) {
        Report-Pass "CLI Flag --help" "Usage text rendered correctly."
    } else {
        Report-Fail "CLI Flag --help" "Exit code: $($resHelp.ExitCode). Output: $($resHelp.Output)"
    }

    # Check 2: --version
    $resVer = Invoke-NativeCli $BinPath @("--version")
    if ($resVer.ExitCode -eq 0 -and ($resVer.Output -match "0.1.0" -or $resVer.Output -match "aurazone_ripple")) {
        Report-Pass "CLI Flag --version" "Reported: $($resVer.Output.Trim())"
    } else {
        Report-Fail "CLI Flag --version" "Exit code: $($resVer.ExitCode). Output: $($resVer.Output)"
    }

    # Check 3: --mock --help
    $resMock = Invoke-NativeCli $BinPath @("--mock", "--help")
    if ($resMock.ExitCode -eq 0) {
        Report-Pass "CLI Flag --mock accepted" "Dry-run flag acknowledged."
    } else {
        Report-Fail "CLI Flag --mock accepted" "Exit code: $($resMock.ExitCode)"
    }

    # Check 4: Conflicting flags validation (--daemon --stop)
    $resConflict = Invoke-NativeCli $BinPath @("--daemon", "--stop")
    if ($resConflict.ExitCode -eq 2) {
        Report-Pass "CLI Conflicting Flags Rejection" "Exited with code 2 as expected."
    } else {
        Report-Fail "CLI Conflicting Flags Rejection" "Expected code 2, got $($resConflict.ExitCode)"
    }

    # Check 5: Unrecognized flag validation
    $resUnrec = Invoke-NativeCli $BinPath @("--unknown-flag-xyz")
    if ($resUnrec.ExitCode -eq 2) {
        Report-Pass "CLI Unrecognized Flag Rejection" "Exited with code 2 as expected."
    } else {
        Report-Fail "CLI Unrecognized Flag Rejection" "Expected code 2, got $($resUnrec.ExitCode)"
    }
}

# =========================================================================
# Phase 3: Zombie & Lingering Daemon Process Verification
# =========================================================================
Write-Host ""
Write-Host "[3/3] Zombie Process & Process Leak Verification..." -ForegroundColor Yellow

$Zombies = Get-Process -Name "aurazone_ripple" -ErrorAction SilentlyContinue
if ($Zombies) {
    $ZombieCount = ($Zombies | Measure-Object).Count
    Report-Fail "Zombie Process Cleanliness" "Detected $ZombieCount orphaned aurazone_ripple process(es)!"
    Write-Host "  Terminating orphaned processes to ensure clean environment..." -ForegroundColor Yellow
    foreach ($p in $Zombies) {
        try {
            $p.Kill()
            Write-Host "    Killed PID $($p.Id)" -ForegroundColor DarkGray
        } catch {}
    }
} else {
    Report-Pass "Zero Zombie Processes" "No lingering background daemon processes detected."
}

# =========================================================================
# Summary & Strict Non-Deceptive Exit Code Semantics
# =========================================================================
$TotalStartTime.Stop()
$ElapsedSec = [Math]::Round($TotalStartTime.Elapsed.TotalSeconds, 2)

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  E2E TEST RUNNER SUMMARY                                 " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Passed  : $PassCount" -ForegroundColor Green
Write-Host "  Failed  : $FailCount" -ForegroundColor $(if ($FailCount -eq 0) { "Green" } else { "Red" })
Write-Host "  Skipped : $SkippedCount" -ForegroundColor $(if ($SkippedCount -eq 0) { "Green" } else { "Yellow" })
Write-Host "  Elapsed : $ElapsedSec s" -ForegroundColor DarkGray

if ($MissingPrereqs.Count -gt 0 -or $SkippedCount -gt 0) {
    Write-Host ""
    Write-Host "  RESULT: FAILED (Tests were skipped / prerequisites missing)" -ForegroundColor Red
    if ($MissingPrereqs.Count -gt 0) {
        Write-Host "  Missing prerequisites: $($MissingPrereqs -join ', ')" -ForegroundColor DarkRed
    }
    exit 1
} elseif ($FailCount -gt 0) {
    Write-Host ""
    Write-Host "  RESULT: FAILED ($FailCount checks failed)" -ForegroundColor Red
    exit 1
} elseif ($PassCount -eq 0) {
    Write-Host ""
    Write-Host "  RESULT: FAILED (No test assertions executed)" -ForegroundColor Red
    exit 1
} else {
    Write-Host ""
    Write-Host "  RESULT: SUCCESS (All checks passed cleanly)" -ForegroundColor Green
    exit 0
}
