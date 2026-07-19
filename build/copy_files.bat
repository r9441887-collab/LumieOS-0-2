@echo off
echo Creating directories...
mkdir F:\EFI\BOOT 2>nul

echo Copying BOOTX64.EFI...
copy /Y "C:\Users\user\Desktop\os66\target\x86_64-unknown-uefi\release\lumieos-loader.efi" "F:\EFI\BOOT\BOOTX64.EFI"

echo Copying install.pkg...
copy /Y "C:\Users\user\Desktop\os66\build\install.pkg" "F:\install.pkg"

echo.
echo === Verify ===
dir F:\
dir F:\EFI\BOOT
