@echo off
title Pasang Aura Ripple di Windows Startup
echo Mendaftarkan Scheduled Task 'AuraZoneRipple' agar aktif otomatis saat Windows login...
schtasks /create /tn "AuraZoneRipple" /tr "\"C:\Users\ROG\AppData\Local\Programs\Python\Python313\pythonw.exe\" \"C:\Users\ROG\AuraZoneRipple\rog_aura_ripple.py\" --daemon" /sc onlogon /rl highest /f
if %errorlevel% == 0 (
    echo.
    echo [SUKSES] Aura Zone Ripple telah didaftarkan ke Startup Task Scheduler!
    echo Efek riak akan otomatis berjalan setiap kali Anda masuk ke Windows.
) else (
    echo.
    echo [GAGAL] Gagal membuat task. Pastikan dijalankan sebagai Administrator jika diperlukan.
)
pause
