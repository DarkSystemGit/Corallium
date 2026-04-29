@echo off
setlocal enabledelayedexpansion

echo Building Corallium for Windows...
cargo build --release --quiet

if errorlevel 1 (
    echo Build failed
    exit /b 1
)

set INSTALL_DIR=C:\Program Files\Corallium

echo Installing to %INSTALL_DIR%...

if not exist "%INSTALL_DIR%" (
    mkdir "%INSTALL_DIR%"
)

echo Copying binary...
copy "target\release\Corallium.exe" "%INSTALL_DIR%\corallium.exe"

echo Copying standard library...
if exist "%INSTALL_DIR%\std" (
    rmdir /s /q "%INSTALL_DIR%\std"
)
xcopy "src\std" "%INSTALL_DIR%\std" /E /I /Y

echo.
echo Installation complete!
echo.
echo To add Corallium to PATH:
echo   1. Open Environment Variables (search in Start menu)
echo   2. Click "Environment Variables..."
echo   3. Under "User variables", select "Path" and click "Edit"
echo   4. Click "New" and add: %INSTALL_DIR%
echo   5. Click OK on all dialogs
echo.
echo After adding to PATH, restart your terminal and run:
echo   corallium run --file myprogram.coral
