#!/bin/bash
# keys.sh monsock tok... ; token "k:NAME" sends qemu key NAME; other tokens are typed char by char (US layout)
S=$(dirname "$0"); m=$1; shift
declare -A K=( [" "]=spc ["-"]=minus ["_"]=shift-minus ["$"]=shift-4 ["/"]=slash ["."]=dot [","]=comma [";"]=semicolon
  [":"]=shift-semicolon ["'"]=apostrophe ['"']=shift-apostrophe [">"]=shift-dot ["<"]=shift-comma ["~"]=shift-grave_accent
  ["\`"]=grave_accent ["|"]=shift-backslash ["\\"]=backslash ["="]=equal ["+"]=shift-equal ["&"]=shift-7 ["*"]=shift-8
  ["("]=shift-9 [")"]=shift-0 ["!"]=shift-1 ["@"]=shift-2 ["#"]=shift-3 ["%"]=shift-5 ["^"]=shift-6 ["?"]=shift-slash
  ["["]=bracket_left ["]"]=bracket_right ["{"]=shift-bracket_left ["}"]=shift-bracket_right )
for w in "$@"; do
  if [[ $w == k:* ]]; then python3 -I $S/mon.py $m sendkey "${w#k:}"; continue; fi
  for ((i=0;i<${#w};i++)); do c=${w:i:1}
    if [[ -n ${K[$c]+x} ]]; then c=${K[$c]}; elif [[ $c == [A-Z] ]]; then c=shift-${c,,}; fi
    python3 -I $S/mon.py $m sendkey "$c"; done
done
