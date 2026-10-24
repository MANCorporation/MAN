#!/bin/bash
# flow.sh: click through DUHACH! up to the confirmation (VM T)
R=/tmp/claude-1000/winvm; S=/tmp/claude-1000/-home-kristihack-Documents-MANRUST/6669f544-3acf-4c2d-bf09-25e9e377451d/scratchpad
q() { python3 -I $S/qmp.py $R/T.qmp click "$1" "$2"; sleep "${3:-2}"; }
shot() { python3 -I $S/mon.py $R/T.mon screendump $R/$1.ppm; python3 $S/ppm2png.py $R/$1.ppm $R/$1.png >/dev/null; }
sleep 25; shot f1
q 538 686            # Get Started
q 778 324 3          # Choose…
python3 -I /tmp/claude-1000/work/type.py $R/T.mon 'C:\Users\man\Downloads\MAN-10.0-amd64.iso
'
sleep 20; shot f2
q 538 686            # Next (ISO)
shot f3
q 538 686            # Next (space)
q 797 578 1          # backup switch
q 538 686            # Prepare MAN
shot f4
q 622 443 1          # confirm
