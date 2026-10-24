# Test agent: runs base64-encoded PowerShell commands received on COM1 and
# sends the output back (QEMU serial socket on the host, see winvm.sh).
$ErrorActionPreference = 'Continue'
while ($true) {
    try {
        $p = New-Object System.IO.Ports.SerialPort 'COM1', 115200, 'None', 8, 'One'
        $p.NewLine = "`n"; $p.ReadTimeout = -1; $p.Open()
        $p.WriteLine('AGENT-READY')
        while ($true) {
            $line = $p.ReadLine().Trim()
            if (-not $line) { continue }
            try {
                $cmd = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($line))
                $out = (Invoke-Expression $cmd 2>&1 | Out-String -Width 300)
            } catch { $out = "AGENT-ERROR: $_" }
            $p.WriteLine('BEGIN ' + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($out)) + ' END')
        }
    } catch { Start-Sleep 3 }
    finally { if ($p) { $p.Close() } }
}
