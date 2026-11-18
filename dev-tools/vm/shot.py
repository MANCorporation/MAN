# shot.py MONITOR-SOCKET OUT.png: screenshot of a QEMU VM as PNG.
import socket, sys, time, os, subprocess
ppm = sys.argv[2] + ".ppm"
s = socket.socket(socket.AF_UNIX); s.connect(sys.argv[1]); time.sleep(0.2); s.recv(65536)
s.sendall(f"screendump {ppm}\n".encode()); time.sleep(0.8)
subprocess.run([sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "ppm2png.py"), ppm, sys.argv[2]], stdout=subprocess.DEVNULL, check=True)
os.remove(ppm)
