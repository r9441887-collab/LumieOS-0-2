@echo off
echo Stopping Ext2Fsd...
sc stop Ext2Fsd
sc stop ext2fsd
timeout /t 2

echo.
echo Stopping all Ext2Fsd related services...
taskkill /f /im ext2mount.exe 2>nul
taskkill /f /im ext2srv.exe 2>nul
timeout /t 1

echo.
echo Unassigning F:
mountvol F: /D 2>nul

echo.
echo Listing volumes...
mountvol

echo.
echo Trying to assign F: to USB volume...
mountvol F: \\?\Volume{fa5f66c5-8286-11f1-b465-00e04f176552}\

echo.
echo Checking F:\
dir F:\

echo.
echo Restarting Ext2Fsd...
sc start Ext2Fsd
sc start ext2fsd
