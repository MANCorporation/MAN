#!/bin/bash
# click.sh mon x y  -- click at screen pixel: home the pointer at (0,0), then move relatively
S=$(dirname "$0"); m=$1
python3 -I $S/mon.py $m mouse_move -4000 -4000; sleep 0.2
python3 -I $S/mon.py $m mouse_move $2 $3; sleep 0.3
python3 -I $S/mon.py $m mouse_button 1; sleep 0.1; python3 -I $S/mon.py $m mouse_button 0
