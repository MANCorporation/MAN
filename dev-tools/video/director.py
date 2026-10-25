# director.py: film a scene in a QEMU VM. Records the screen (QMP screendump,
# PPM frames + times.txt) while moving the mouse along smooth eased paths and
# logging every pointer position and click to cursor.json, so the editor can
# draw a cursor on top (screendump does not include the pointer).
#
#   from director import Scene
#   with Scene("VD.qmp", "out/sF", 1920, 1080) as s:
#       s.move(900, 500, 0.8); s.click(); s.type("sudoku"); s.wait(2)
import json, math, os, socket, threading, time

KEYS = {' ': 'spc', '-': 'minus', '.': 'dot', '/': 'slash', '\n': 'ret'}


class Qmp:
    """One connection (QEMU's QMP socket takes a single client), shared by the
    recorder thread and the input calls."""
    def __init__(self, path):
        self.lock = threading.Lock()
        self.s = socket.socket(socket.AF_UNIX)
        self.s.connect(path)
        self.f = self.s.makefile('rw')
        self.f.readline()
        self._cmd("qmp_capabilities")

    def cmd(self, c, a=None):
        with self.lock:
            return self._cmd(c, a)

    def _cmd(self, c, a=None):
        self.f.write(json.dumps({"execute": c, **({"arguments": a} if a else {})}) + "\n")
        self.f.flush()
        while True:
            r = json.loads(self.f.readline())
            if 'return' in r or 'error' in r:
                return r


def ease(t):  # smooth in-out
    return t * t * (3 - 2 * t)


class Scene:
    def __init__(self, sock, out, w=1920, h=1080, fps=15, start=(960, 600)):
        self.sock, self.out, self.w, self.h, self.fps = sock, out, w, h, fps
        self.x, self.y = start
        self.log = []
        os.makedirs(out, exist_ok=True)

    # ── recording ──
    def _record(self):
        q = self.q
        n = 0
        times = []
        while not self.stop:
            t = time.time()
            q.cmd("screendump", {"filename": f"{self.out}/f{n:05d}.ppm", "format": "ppm"})
            times.append(t - self.t0)
            n += 1
            time.sleep(max(0, 1 / self.fps - (time.time() - t)))
        with open(f"{self.out}/times.txt", "w") as f:
            f.write("\n".join(f"{x:.3f}" for x in times))

    def __enter__(self):
        self.q = Qmp(self.sock)
        self._abs(self.x, self.y)
        self.stop = False
        self.t0 = time.time()
        self.rec = threading.Thread(target=self._record)
        self.rec.start()
        self._mark("pos")
        return self

    def __exit__(self, *a):
        self.stop = True
        self.rec.join()
        with open(f"{self.out}/cursor.json", "w") as f:
            json.dump(self.log, f)

    # ── input ──
    def _mark(self, kind):
        self.log.append({"t": round(time.time() - self.t0, 3), "x": round(self.x), "y": round(self.y), "k": kind})

    def _abs(self, x, y):
        self.q.cmd("input-send-event", {"events": [
            {"type": "abs", "data": {"axis": "x", "value": int(x * 32767 / self.w)}},
            {"type": "abs", "data": {"axis": "y", "value": int(y * 32767 / self.h)}}]})

    def move(self, x, y, dur=0.7):
        x0, y0 = self.x, self.y
        # A slight arc looks like a hand, not a robot.
        bend = 0.12 * math.hypot(x - x0, y - y0)
        nx, ny = (-(y - y0), x - x0)
        ln = math.hypot(nx, ny) or 1
        steps = max(2, int(dur * 60))
        for i in range(1, steps + 1):
            t = ease(i / steps)
            arc = math.sin(math.pi * t) * bend
            self.x = x0 + (x - x0) * t + nx / ln * arc
            self.y = y0 + (y - y0) * t + ny / ln * arc
            self._abs(self.x, self.y)
            self._mark("pos")
            time.sleep(dur / steps)

    def click(self, double=False):
        for _ in range(2 if double else 1):
            self._mark("click")
            self.q.cmd("input-send-event", {"events": [{"type": "btn", "data": {"down": True, "button": "left"}}]})
            time.sleep(0.06)
            self.q.cmd("input-send-event", {"events": [{"type": "btn", "data": {"down": False, "button": "left"}}]})
            time.sleep(0.08)

    def key(self, *names):
        self.q.cmd("send-key", {"keys": [{"type": "qcode", "data": n} for n in names]})
        time.sleep(0.05)

    def type(self, text, cps=9):
        for ch in text:
            name = KEYS.get(ch, ch.lower())
            if ch.isupper():
                self.key('shift', name)
            else:
                self.key(name)
            time.sleep(1 / cps)

    def wait(self, s):
        end = time.time() + s
        while time.time() < end:
            self._mark("pos")
            time.sleep(0.1)
