@echo off
title Hapus Aura Ripple dari Windows Startup
echo Menghapus Scheduled Task 'AuraZoneRipple'...
schtasks /delete /tn "AuraZoneRipple" /f
if %errorlevel% == 0 (
    echo.
    echo [SUKSES] Task 'AuraZoneRipple' berhasil dihapus dari startup.
) else (
    echo.
    echo [INFO] Task tidak ditemukan atau sudah dihapus sebelumnya.
)
pause
