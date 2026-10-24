#!/bin/bash
# tryapps.sh mon prefix name... : launch each via overview search, screenshot, close
S=$(dirname "$0"); m=$1; pre=$2; shift 2; i=0
for name in "$@"; do i=$((i+1))
  timeout 10 $S/keys.sh $m k:meta_l; sleep 1.5; timeout 40 $S/keys.sh $m "$name"; sleep 1.5; timeout 10 $S/keys.sh $m k:ret; sleep 12
  python3 -I $S/mon.py $m screendump $S/$pre$i.ppm; timeout 10 $S/keys.sh $m k:alt-f4; sleep 2
done
python3 -I $S/sheet.py $S/$pre.png $(for j in $(seq $i); do echo $S/$pre$j.ppm; done)
