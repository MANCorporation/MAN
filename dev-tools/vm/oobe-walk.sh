#!/bin/bash
# oobe-walk.sh NAME : drive MAN Setup with dark+green+wallpaper, location on, encryption with disk-pass-77
S=$(cd "$(dirname "$0")" && pwd); cd $S; N=$1; q(){ python3 -I qmp.py $N.qmp "$@"; }
q click 640 529; sleep 2; q scroll 640 450 15; sleep 1; q click 496 591; sleep 1; q click 640 649; sleep 4
q click 640 649; sleep 2
timeout 60 ./keys.sh $N.mon "Kristi Hack"; q click 600 494; sleep 1; timeout 60 ./keys.sh $N.mon kristipass1; q click 600 549; sleep 1; timeout 60 ./keys.sh $N.mon kristipass1; sleep 1
q click 640 649; sleep 2; q click 742 412; sleep 1; q click 547 582; sleep 1; q scroll 640 450 12; sleep 2; q click 827 552; sleep 2
q click 640 649; sleep 2; q click 878 359; sleep 1; q click 640 649; sleep 2
q click 853 352; sleep 1.5; q click 600 407; sleep 1; timeout 60 ./keys.sh $N.mon disk-pass-77; q click 600 462; sleep 1; timeout 60 ./keys.sh $N.mon disk-pass-77; sleep 1
q click 640 649
