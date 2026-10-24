#!/bin/bash
# swap.sh: rebuild DUHACH! for Windows, put the exe into the test VM T and start it
set -e
W=/tmp/claude-1000/work; R=/tmp/claude-1000/winvm; S=/tmp/claude-1000/-home-kristihack-Documents-MANRUST/6669f544-3acf-4c2d-bf09-25e9e377451d/scratchpad
~/Documents/MANRUST/duhach/build-windows.sh 2>&1 | grep -E "^error|Built" -A5
rm -rf $W/upd && mkdir $W/upd && cp ~/Documents/MANRUST/duhach/target/x86_64-pc-windows-gnu/release/duhach.exe "$W/upd/DUHACH!.exe"
xorriso -as mkisofs -quiet -J -joliet-long -r -V UPD -o $W/upd.iso $W/upd 2>/dev/null
python3 -I $S/mon.py $R/T.mon eject -f cd1; sleep 1; python3 -I $S/mon.py $R/T.mon change cd1 $W/upd.iso; sleep 4
python3 -I ~/Documents/MANRUST/duhach/testvm/winctl.py $R/T.com 'Get-Process DUHACH* -EA SilentlyContinue | Stop-Process -Force; Start-Sleep 1; $d = (Get-Volume | ? FileSystemLabel -eq "UPD").DriveLetter; Copy-Item -Force "$($d):\DUHACH!.exe" "C:\Users\man\Desktop\DUHACH!\DUHACH!.exe"; attrib -r "C:\Users\man\Desktop\DUHACH!\DUHACH!.exe"; Start-Process -FilePath "C:\Users\man\Desktop\DUHACH!\DUHACH!.exe" -WorkingDirectory "C:\Users\man\Desktop\DUHACH!"; "started"' 40
