@echo off
echo ========================================================
echo  Installing AuraZoneRipple to Windows Task Scheduler...
echo ========================================================
schtasks /create /tn "AuraZoneRipple" /tr "\"%~dp0target\release\aurazone_ripple.exe\" --daemon" /sc ONLOGON /rl HIGHEST /f
if %errorlevel% equ 0 (
    echo.
    echo [OK] Task Scheduler registered successfully with Highest Privileges!
    reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v "AuraZoneRipple" /f >nul 2>&1
    echo [OK] Removed from legacy HKCU Run key to eliminate Windows Startup Delay.
    echo [OK] AuraZone will now start instantly at logon with zero delay!
) else (
    echo.
    echo [ERROR] Gagal mendaftarkan task.
    echo Silakan KLIK KANAN file ini lalu pilih 'Run as administrator'!
)
pause
