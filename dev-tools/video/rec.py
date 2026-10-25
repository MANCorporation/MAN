# rec.py QMP_SOCK OUTDIR SECONDS [FPS]: record the VM screen as numbered PNG
# frames (QMP screendump), with a times.txt of capture times for ffmpeg.
import json, os, socket, sys, time

sock, out, secs = sys.argv[1], sys.argv[2], float(sys.argv[3])
fps = float(sys.argv[4]) if len(sys.argv) > 4 else 12
os.makedirs(out, exist_ok=True)
s = socket.socket(socket.AF_UNIX)
s.connect(sock)
f = s.makefile('rw')
f.readline()


def cmd(c, a=None):
    f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n")
    f.flush()
    while True:
        r = json.loads(f.readline())
        if 'return' in r or 'error' in r:
            return r


cmd("qmp_capabilities")
start = time.time()
n = 0
times = []
while time.time() - start < secs:
    t = time.time()
    r = cmd("screendump", {"filename": f"{out}/f{n:05d}.ppm", "format": "ppm"})
    if 'error' in r:
        sys.exit(r['error'])
    times.append(t - start)
    n += 1
    time.sleep(max(0, 1 / fps - (time.time() - t)))
with open(f"{out}/times.txt", "w") as tf:
    tf.write("\n".join(f"{x:.3f}" for x in times))
print(n, "frames", f"{n / secs:.1f} fps")
