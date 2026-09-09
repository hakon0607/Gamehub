@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"

REM ===========================================================================
REM  Builds GameHub into a .exe on this PC.
REM
REM  Double-click it, or run it from Command Prompt. It checks the tools it
REM  needs first and stops with a plain explanation if one is missing, rather
REM  than failing halfway through with a wall of red text.
REM
REM  Nothing is downloaded or installed. Nothing leaves this PC.
REM ===========================================================================

echo.
echo   Building GameHub
echo   ================
echo.

REM --- The tools, checked before anything slow starts ------------------------

set MISSING=

where node >nul 2>&1 || set MISSING=!MISSING! node
where cargo >nul 2>&1 || set MISSING=!MISSING! rust

REM pnpm may be a shim on PATH or only reachable through corepack.
set PNPM=pnpm
where pnpm >nul 2>&1 || (
  corepack --version >nul 2>&1 && (
    echo   Turning on pnpm through corepack...
    call corepack enable >nul 2>&1
    call corepack prepare pnpm@9.12.0 --activate >nul 2>&1
  )
)
where pnpm >nul 2>&1 || set MISSING=!MISSING! pnpm

if not "!MISSING!"=="" goto :missing

REM --- The Microsoft C++ linker, which Rust needs on Windows -----------------
REM  Checked separately because its absence otherwise surfaces as
REM  "link.exe not found" after several minutes of compiling.

where link.exe >nul 2>&1
if errorlevel 1 (
  where cl.exe >nul 2>&1
  if errorlevel 1 (
    echo   The Microsoft C++ linker was not found on PATH.
    echo.
    echo   It is installed but only on PATH inside its own prompt. Open
    echo   "x64 Native Tools Command Prompt for VS" from the Start Menu,
    echo   change to this folder, and run BUILD.bat there.
    echo.
    echo   If it is genuinely not installed, get "Build Tools for Visual Studio"
    echo   and tick "Desktop development with C++".
    echo.
    goto :stop
  )
)

REM --- Build ----------------------------------------------------------------

echo   1/4  Installing packages...
call %PNPM% install
if errorlevel 1 goto :failed

echo.
echo   2/4  Getting ffmpeg for Replay...
if exist "apps\desktop\src-tauri\resources\ffprobe.exe" (
  echo        Already here, skipping.
) else (
  REM Replay is the only feature that needs this. A failure here is not worth
  REM stopping the build for - everything else works without it.
  call %PNPM% fetch-ffmpeg
  if errorlevel 1 echo        Could not download it. GameHub will build fine; Replay will not record.
)

echo.
echo   3/4  Building the interface...
call %PNPM% --filter @gamehub/desktop build
if errorlevel 1 goto :failed

echo.
echo   4/4  Building the app. First time takes a while - Rust compiles
echo        everything once, then caches it. Later builds are much faster.
echo.
call %PNPM% --filter @gamehub/desktop tauri build
if errorlevel 1 goto :failed

REM --- Collect the results somewhere obvious --------------------------------

set OUT=%~dp0GameHub-ferdig
if not exist "%OUT%" mkdir "%OUT%"

set BUNDLE=apps\desktop\src-tauri\target\release
set FOUND=

for %%F in ("%BUNDLE%\bundle\nsis\*-setup.exe") do (
  copy /y "%%F" "%OUT%\GameHub-Setup.exe" >nul && set FOUND=1
)
if exist "%BUNDLE%\GameHub.exe" (
  copy /y "%BUNDLE%\GameHub.exe" "%OUT%\GameHub.exe" >nul && set FOUND=1
  REM The portable copy needs its resources beside it; the installer bundles
  REM them itself. Without this, Replay cannot find ffmpeg.
  if exist "apps\desktop\src-tauri\resources" (
    xcopy /y /q "apps\desktop\src-tauri\resources\*" "%OUT%\resources\" >nul 2>&1
  )
)

if not defined FOUND goto :nooutput

echo.
echo   Done.
echo.
echo   Your files are in:  %OUT%
echo.
echo     GameHub-Setup.exe   Run this once. Installs GameHub properly, with a
echo                         Start Menu entry. This is the one to use.
echo.
echo     GameHub.exe         The same app without installing. Keep it in this
echo                         folder - the resources folder next to it belongs
echo                         to it.
echo.
echo   Windows may warn that the publisher is unknown, because the file is not
echo   code-signed. Choose "More info" then "Run anyway".
echo.
explorer "%OUT%"
goto :stop

REM --- Anything that went wrong ---------------------------------------------

:missing
echo   Cannot build yet. Missing:!MISSING!
echo.
echo     node   https://nodejs.org  (choose the LTS version)
echo     rust   https://rustup.rs
echo     pnpm   comes with Node - run:  corepack enable
echo.
echo   Install what is listed, close this window, open it again, and run
echo   BUILD.bat once more.
echo.
goto :stop

:failed
echo.
echo   The build stopped. The last lines above say why.
echo.
echo   Two things fix most failures:
echo     - Close GameHub if it is running. A running .exe cannot be replaced.
echo     - Delete the folder apps\desktop\src-tauri\target and run this again.
echo.
goto :stop

:nooutput
echo.
echo   The build reported success but no .exe was found in %BUNDLE%.
echo   Check the output above for a bundling error.
echo.

:stop
echo.
pause
endlocal
