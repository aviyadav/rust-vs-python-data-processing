@echo off
REM ============================================================================
REM  run_all.bat  —  Build, start, test, and compare both FastAPI versions
REM ============================================================================
REM
REM  This script:
REM    1. Builds the Rust validator (cargo build --release)
REM    2. Runs the BEFORE (vulnerable) tests
REM    3. Starts the Rust validator
REM    4. Runs the AFTER (secure) tests
REM    5. Stops all services
REM
REM  Prerequisites: Rust (cargo), Python 3.10+, pip
REM ============================================================================

setlocal enabledelayedexpansion

REM ── Ensure we run from the script's own directory ───────────────────────
cd /d "%~dp0"

echo.
echo ============================================================
echo   Rust Security Layer for FastAPI  —  Full Test Suite
echo ============================================================
echo.

REM ── Step 1: Install Python dependencies ──────────────────────────────
echo [1/5] Installing Python dependencies...
pip install -r before\requirements.txt -q 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: pip install failed for before/requirements.txt
    exit /b 1
)
pip install -r after\fastapi_app\requirements.txt -q 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: pip install failed for after/fastapi_app/requirements.txt
    exit /b 1
)
echo        Done.
echo.

REM ── Step 2: Build Rust validator ─────────────────────────────────────
echo [2/5] Building Rust validator (release mode)...
if not exist "after\rust_validator\Cargo.toml" (
    echo ERROR: after\rust_validator\Cargo.toml not found!
    exit /b 1
)
pushd after\rust_validator
cargo build --release 2>&1
set BUILD_RESULT=%ERRORLEVEL%
popd
if %BUILD_RESULT% neq 0 (
    echo ERROR: Rust build failed (exit code %BUILD_RESULT%)!
    exit /b 1
)
echo        Rust validator built successfully.
echo.

REM ── Step 3: Test BEFORE (vulnerable) app ────────────────────────────
echo [3/5] Testing BEFORE (vulnerable) FastAPI app...
echo        ----------------------------------------
python before\test_before.py
if %ERRORLEVEL% neq 0 (
    echo WARNING: Before tests had failures (exit code %ERRORLEVEL%).
)
echo.

REM ── Step 4: Start Rust validator ─────────────────────────────────────
echo [4/5] Starting Rust validator on port 3000...

set RUST_EXE=after\rust_validator\target\release\rust-validator.exe
if not exist "%RUST_EXE%" (
    echo ERROR: Rust binary not found at %RUST_EXE%
    echo        Make sure 'cargo build --release' succeeded.
    exit /b 1
)

REM Start the Rust validator in the background (no new window).
REM Use empty window-title "" so the quoted path is treated as the command.
start "" /B "%RUST_EXE%"

REM Wait up to 10 seconds for port 3000 to become available.
set /a ATTEMPTS=0
:wait_rust
ping -n 2 127.0.0.1 >nul
set /a ATTEMPTS+=1
netstat -ano 2>nul | findstr /R ":3000 .*LISTENING" >nul
if !ERRORLEVEL! equ 0 goto rust_ready
if !ATTEMPTS! lss 10 goto wait_rust
echo ERROR: Rust validator did not start within 10 seconds!
exit /b 1

:rust_ready
echo        Rust validator is running on port 3000.
echo.

REM ── Step 5: Test AFTER (secure) app ──────────────────────────────────
echo [5/5] Testing AFTER (secure) FastAPI app...
echo        ----------------------------------------
python after\fastapi_app\test_after.py
set TEST_RESULT=%ERRORLEVEL%
echo.

REM ── Cleanup: stop the Rust validator ──────────────────────────────────
echo Stopping Rust validator...

REM Method 1: kill by PID listening on port 3000
for /f "tokens=5" %%a in ('netstat -ano 2^>nul ^| findstr /R ":3000 .*LISTENING"') do (
    set RUST_PID=%%a
)
if defined RUST_PID (
    echo        Killing PID %RUST_PID% ...
    taskkill /PID %RUST_PID% /F >nul 2>&1
    echo        Rust validator stopped.
) else (
    echo        (No process found on port 3000 — already stopped?)
)

echo.
if %TEST_RESULT% equ 0 (
    echo ============================================================
    echo   ^^! ALL TESTS PASSED
    echo ============================================================
) else (
    echo ============================================================
    echo   ^^! SOME TESTS FAILED (exit code %TEST_RESULT%)
    echo ============================================================
)
echo.
exit /b %TEST_RESULT%
