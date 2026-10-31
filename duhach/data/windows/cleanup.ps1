# DUHACH! cleanup, run by the "DUHACH! cleanup" task at every Windows start.
# Once MAN is installed (MAN removes its installer partition), the one-time
# "MAN Installer" boot entry is useless: Windows would keep restoring it into
# the firmware boot menu from its boot database. Delete it, then this task.
$dir = Join-Path $env:ProgramData 'DUHACH!'
$state = Join-Path $dir 'state.json'
if (@(Get-Volume | Where-Object FileSystemLabel -eq 'MAN_LIVE').Count -gt 0) { exit 0 }
if (Test-Path -LiteralPath $state) {
    $s = Get-Content -LiteralPath $state -Raw | ConvertFrom-Json
    if ($s.boot_entry) { bcdedit /delete $s.boot_entry | Out-Null }
    # DUHACH!'s loader on the EFI system partition.
    $l = [char[]](90..68) | ? { -not (Test-Path "$($_):\") } | select -First 1
    mountvol "$($l):" /S 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) { Remove-Item -Recurse -Force -LiteralPath "$($l):\EFI\MAN-Installer" -EA SilentlyContinue; mountvol "$($l):" /D | Out-Null }
    Remove-Item -LiteralPath $state -Force
}
Unregister-ScheduledTask -TaskName 'DUHACH! cleanup' -Confirm:$false -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $dir 'cleanup.ps1') -Force -ErrorAction SilentlyContinue
