#!/bin/bash
# dock-vs-stock.sh T-DIR W H NAME: in the running test VM, same scene with stock dash and MAN Dock.
T=$1 W=$2 H=$3 N=$4; S=$(dirname "$(readlink -f "$0")"); [ -f "$S/mon.py" ] || S=$SCR; O=/tmp/claude-1000; A=$(cd "$S/../arm64" 2>/dev/null && pwd); A=${A:-$ARM}
shot(){ python3 -I $S/mon.py $T/m screendump $O/$1.ppm >/dev/null; python3 -I $S/ppm2png.py $O/$1.ppm $O/$1.png >/dev/null; }
R(){ (cd $A; timeout 45 python3 vmrun.py $T/s 40 "$1") | grep -v "Done  \|^\[[0-9]\]"; }
L="man-look@man.invalid"; D="man-dock@man.invalid"; B="man-blur@man.invalid"
R ". /mnt/w/env.sh; pkill -x man-installer; pkill gnome-text; asuser gsettings set org.gnome.shell enabled-extensions \"['$L']\"; sleep 4; asuser nohup setsid gnome-text-editor >/tmp/t.log 2>&1 & sleep 4; echo a" | tail -1
python3 $S/mv.py $T/q $W $H 200 300; sleep 1 2>/dev/null
python3 $S/key.py $T/m esc; timeout 3 sh -c 'sleep 2'; python3 $S/key.py $T/m meta_l; timeout 4 sh -c 'sleep 2.5'; shot ${N}s; python3 $S/key.py $T/m esc; timeout 3 sh -c 'sleep 1.5'
R ". /mnt/w/env.sh; asuser gsettings set org.gnome.shell enabled-extensions \"['$L','$D','$B']\"; sleep 6; echo b" | tail -1
python3 $S/key.py $T/m meta_l; timeout 4 sh -c 'sleep 2.5'; shot ${N}o; python3 $S/key.py $T/m esc; timeout 3 sh -c 'sleep 2'; shot ${N}d
