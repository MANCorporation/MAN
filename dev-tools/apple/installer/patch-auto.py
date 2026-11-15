#!/usr/bin/env python3
"""MAN: fewer manual steps in the Asahi installer (MIT).
- main.py: after the macOS part, shut down by itself after a countdown (Ctrl-C stops it).
- step2/step2.sh (runs in the Mac's start-up recovery): start right away and restart by
  itself when done. Apple still requires holding the power button to enter that mode
  and the macOS password for bputil/kmutil: those cannot be automated.
  patch-auto.py INSTALLER_TREE"""
import sys
t = sys.argv[1]

def patch(path, pairs):
    s = open(path).read()
    for old, new in pairs:
        if old not in s:
            sys.exit(f"patch-auto: pattern not found in {path}:\n{old}")
        s = s.replace(old, new, 1)
    open(path, "w").write(s)
    print("patched", path)

patch(f"{t}/main.py", [
    ('''        p_question( "Press enter to continue.")
        self.input()
        print()
        print()
        print()
        p_message( "When the system shuts down, follow these steps:")''',
     '''        print()
        p_message( "When the system shuts down, follow these steps:")'''),
    ('''        time.sleep(2)
        p_prompt( "Press enter to shut down the system.")
        self.input()
        time.sleep(1)
        os.system("shutdown -h now")''',
     '''        p_message( "In step 7 only your macOS password is needed; the Mac restarts by itself.")
        print()
        # MAN: no extra key press; Ctrl-C stops the countdown.
        try:
            for i in range(60, 0, -1):
                sys.stdout.write(f"\\rShutting down in {i:2d} seconds (Ctrl-C to stop)... ")
                sys.stdout.flush()
                time.sleep(1)
        except KeyboardInterrupt:
            print()
            p_prompt( "Countdown stopped. Press enter to shut down the system.")
            self.input()
        print()
        time.sleep(1)
        os.system("shutdown -h now")'''),
])

patch(f"{t}/step2/step2.sh", [
    ('''echo "Press enter to continue."
echo

read

while ! bputil -nc -v "$VGID"; do''',
     '''echo "Starting in 10 seconds..."
echo
sleep 10

while ! bputil -nc -v "$VGID"; do'''),
    ('''echo "Installation complete! Press enter to reboot."
read

reboot''',
     '''echo "Installation complete! Restarting in 10 seconds..."
sleep 10

reboot'''),
])
