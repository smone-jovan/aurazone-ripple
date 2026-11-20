@echo off
echo ========================================================
echo  Installing Elevated Gaming Startup Task (AuraZoneRipple)
echo ========================================================
"%~dp0target\release\aurazone_ripple.exe" --install-elevated-task
if %errorlevel% equ 0 (
    echo.
    echo [OK] Elevated Gaming Task registered with HighestAvailable privileges!
    echo [OK] AuraZone daemon will now receive keyboard input inside elevated games ^(ZZZ, etc.^).
) else (
    echo.
    echo [ERROR] Gagal mendaftarkan elevated task.
    echo Silakan KLIK KANAN file ini lalu pilih 'Run as administrator'!
)
pause
