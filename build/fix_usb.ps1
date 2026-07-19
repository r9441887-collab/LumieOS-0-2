# Must run as Administrator
$disk = Get-Disk -Number 2
$part = $disk | Get-Partition -PartitionNumber 1

# Try to remove Ext2Fsd drive mapping first
$ext2vol = Get-Volume | Where-Object { $_.FileSystemLabel -eq 'LUMIEOS' }
if ($ext2vol) {
    Write-Host "Found LUMIEOS volume: $($ext2vol.Path)"
    
    # Try assign via WMI
    $drive = Get-WmiObject -Class Win32_Volume | Where-Object { $_.DeviceID -eq $ext2vol.Path }
    if ($drive) {
        $drive.DriveLetter = "Z:"
        $drive.Put() | Out-Null
        Write-Host "Assigned via WMI"
    }
}

# Check result
Start-Sleep -Seconds 2
Write-Host "`n=== After WMI assign ==="
Get-Volume | Format-Table DriveLetter, FileSystemLabel, FileSystem, Size, DriveType -AutoSize

if (Test-Path Z:\) {
    Write-Host "Z: accessible!"
    Get-ChildItem Z:\
} else {
    Write-Host "Z: not accessible, trying subst..."
    # Last resort: subst
    $volPath = $ext2vol.Path
    subst Z: $volPath
    Get-ChildItem Z:\ -ErrorAction SilentlyContinue
}
