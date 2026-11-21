#!/usr/bin/env python3
"""make-showcase.py OUT.mp4 [--preview] — the MAN 10 TEST 7 showcase (about 10 min):
install, MAN Setup, apps, everyday use and safety, one song per chapter, no voice.

Takes are in video/takes/<name> (filmed with live.py: take.mp4 + cursor.json,
times in seconds of the take). Songs are in video/music. Each scene is a list
of (take, from, to, speed); the pointer is drawn from cursor.json (QEMU's
screendump has none). --preview renders small and fast to check the cuts.
Needs ffmpeg (FFMPEG=…, e.g. tools/ffmpeg/ffmpeg).
"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
TAKES = os.path.join(ROOT, 'video', 'takes')
MUSIC = os.path.join(ROOT, 'video', 'music')
A = os.path.join(HERE, 'assets')
FF = os.environ.get('FFMPEG', 'ffmpeg')
FONT = os.path.join(ROOT, 'dev-tools', 'website', 'man-site', 'fonts', 'AdwaitaSans-Regular.ttf')
OUT = sys.argv[1]
PREVIEW = '--preview' in sys.argv
W, H = (960, 540) if PREVIEW else (1920, 1080)
K = W / 1920  # scale for sizes
FPS = 30
BG, ACC = '0x15171c', '0x2ec27e'
WORK = os.environ.get('WORK') or tempfile.mkdtemp(prefix='man-showcase-')
os.makedirs(WORK, exist_ok=True)

# Takes split after filming: name -> list of (file, first take second).
FILES = {'c5': [('a.mp4', 0.0), ('b.mp4', 14890.0)]}

SONG = {
    1: 'Janji - Heroes Tonight (feat. Johnning) Progressive House NCS - Copyright Free Music.mp3',
    2: 'K-391 - Everybody House NCS - Copyright Free Music.mp3',
    3: 'Robin Hustin - On Fire Future House NCS - Copyright Free Music.mp3',
    4: 'Lost Sky - Vision pt. II (feat. She Is Jules) Future Trap NCS - Copyright Free Music.mp3',
    5: 'Lost Sky - Vision Dubstep NCS - Copyright Free Music.mp3',
}
CREDITS = ['Janji – Heroes Tonight (feat. Johnning)', 'K-391 – Everybody', 'Robin Hustin – On Fire',
           'Lost Sky – Vision pt. II (feat. She Is Jules)', 'Lost Sky – Vision',
           'Music provided by NoCopyrightSounds']

# Each chapter's pieces are sped up by this much more, for a video of about 10 min.
CH_SPEED = {1: 1.05, 2: 1.45, 3: 2.0, 4: 1.55, 5: 1.6}

# ── storyboard ─────────────────────────────────────────────────────────
# ('title', chapter, heading, subheading)  |  ('scene', caption, [(take, a, b, speed), …])
STORY = {
    1: [
        ('title', '1', 'Get MAN', 'From a USB stick to your computer'),
        ('scene', 'Start your computer from the MAN USB stick', [('c1', 0.5, 7.5, 1.0)]),
        ('scene', 'Try MAN first, straight from the stick', [('c1', 7.5, 34.0, 1.6)]),
        ('scene', 'Install MAN: pick the disk', [('c1', 33.5, 45.0, 1.2)]),
        ('scene', 'Your time zone, and a quick keyboard test', [('c1', 45.0, 61.0, 1.3)]),
        ('scene', 'Check everything, then install', [('c1', 61.0, 76.0, 1.2)]),
        ('scene', 'A few minutes later…', [('c1', 76.0, 196.0, 7.0)]),
        ('scene', 'Restart, and remove the stick', [('c2', 268.0, 352.0, 3.0)]),
    ],
    2: [
        ('title', '2', 'Welcome to MAN', 'A short setup, in your language'),
        ('scene', '74 languages to choose from', [('c2', 352.0, 365.0, 1.3)]),
        ('scene', 'Your country', [('c2', 365.0, 384.5, 1.6)]),
        ('scene', 'One short agreement', [('c2', 384.5, 413.0, 2.2)]),
        ('scene', 'Get online', [('c2', 413.0, 427.5, 1.6)]),
        ('scene', 'Your name and password', [('c2', 427.5, 459.5, 1.8)]),
        ('scene', 'Light or dark, your colour, your wallpaper', [('c2', 459.5, 507.0, 1.6)]),
        ('scene', 'Your privacy, your choice', [('c2', 507.0, 524.0, 1.5)]),
        ('scene', 'Lock your disk with a password, if you like', [('c2', 524.0, 539.0, 1.5)]),
        ('scene', 'All done', [('c2', 539.0, 576.0, 2.4)]),
        ('scene', 'Your own login screen', [('c2', 576.0, 662.0, 2.6)]),
    ],
    3: [
        ('title', '3', 'Your Apps', 'Thousands of apps, one click away'),
        ('scene', 'Software: thousands of free apps', [('c3', 46.0, 100.0, 2.2)]),
        ('scene', 'Find an app. Click Install.', [('c3', 100.0, 138.0, 1.5), ('c3', 138.0, 472.0, 30.0)]),
        ('scene', 'Ready to play', [('c3', 472.0, 512.0, 1.6)]),
        ('scene', 'Downloaded an app from a website?', [('c3', 585.0, 632.0, 2.0)]),
        ('scene', 'App Installer guides you, step by step', [('c3', 1097.0, 1126.0, 1.4), ('c3', 1228.0, 1342.0, 4.0)]),
        ('scene', 'Google Chrome, installed', [('c3', 1342.0, 1420.0, 2.4)]),
        ('scene', 'Windows programs too', [('c3', 1492.0, 1524.0, 1.6), ('c3', 2027.0, 2060.0, 1.6)]),
        ('scene', 'Checked for malware before they run', [('c3', 2060.0, 2096.0, 1.6)]),
        ('scene', '7-Zip for Windows, installing on MAN', [('c3', 2096.0, 2196.0, 3.0)]),
        ('scene', 'All your apps in one place', [('c3', 2196.0, 2240.0, 1.4)]),
    ],
    4: [
        ('title', '4', 'Every Day', 'Music, movies, weather and more'),
        ('scene', 'Your music', [('c4', 2.0, 66.0, 2.0)]),
        ('scene', 'Marquee: movies, music and photos on the big screen', [('c4', 70.0, 190.0, 2.6)]),
        ('scene', 'The weather, wherever you are', [('c4', 192.0, 268.0, 2.2)]),
        ('scene', 'Many apps, many desktops', [('c4', 268.0, 333.0, 1.4)]),
        ('scene', 'MAN on the web: mancorporation.com', [('c3', 556.0, 586.0, 1.4)]),
    ],
    5: [
        ('title', '5', 'Safe and Yours', 'Your computer, your way'),
        ('scene', 'Make it yours, live', [('c5', 86.0, 152.0, 1.8)]),
        ('scene', 'Updates arrive by themselves', [('c5', 157.0, 175.0, 1.3), ('c5', 14890.0, 14899.0, 1.3)]),
        ('scene', 'An undo button for your computer', [('c5', 14899.0, 14934.0, 1.3)]),
        ('scene', 'Locked while you are away', [('c5b', 0.0, 4.5, 1.6), ('c5b', 19.0, 44.0, 1.6)]),
        ('scene', 'Hold Alt at start-up: MAN, MAN Recovery, Windows…', [('c5b', 44.0, 149.0, 2.6)]),
        ('scene', 'MAN Recovery helps when something goes wrong', [('c5b', 149.0, 157.0, 0.8)]),
        ('scene', 'Back in seconds', [('c5b', 156.0, 208.0, 1.5)]),
        ('end',),
    ],
}


def ff(*args):
    subprocess.run([FF, '-v', 'error', '-y', *args], check=True)


def enc(path):
    q = ['-preset', 'ultrafast', '-crf', '28'] if PREVIEW else ['-preset', 'medium', '-crf', '17']
    return ['-r', str(FPS), '-c:v', 'libx264', *q, '-pix_fmt', 'yuv420p', path]


def esc(text):  # drawtext text= value inside a filter script
    return text.replace('\\', '\\\\').replace("'", '’').replace(':', '\\:').replace(',', '\\,').replace('%', '\\%')


def source(take, a):
    """(file, seconds into it) for take time a."""
    for name, start in reversed(FILES.get(take, [('take.mp4', 0.0)])):
        if a >= start - 0.01:
            return os.path.join(TAKES, take, name), a - start
    raise ValueError(take, a)


_logs = {}


def cursor_log(take):
    if take not in _logs:
        _logs[take] = json.load(open(os.path.join(TAKES, take, 'cursor.json')))
    return _logs[take]


def segment(i, take, a, b, speed):
    """One piece of a take, with the pointer drawn, sped up, at W x H."""
    out = f'{WORK}/seg_{take}_{a:.2f}_{b:.2f}_{speed}_{W}.mp4'
    if os.path.exists(out):
        return out
    path, off = source(take, a)
    log = cursor_log(take)
    before = [e for e in log if e['t'] <= a]
    x, y = (before[-1]['x'], before[-1]['y']) if before else (log[0]['x'], log[0]['y'])
    cmds, rings, last = [], [], (x, y)
    for e in log:
        if not a < e['t'] < b:
            continue
        t = e['t'] - a
        if (e['x'], e['y']) != last:
            cmds.append(f"{t:.3f} overlay@c x {e['x'] - 8}, overlay@c y {e['y'] - 6};")
            last = (e['x'], e['y'])
        if e['k'] == 'click' and not (rings and t - rings[-1] < 0.25):
            rings.append(t)
            cmds.append(f"{t:.3f} overlay@r x {e['x'] - 60}, overlay@r y {e['y'] - 60};")
    cmdf = f'{WORK}/seg{i}.cmd'
    open(cmdf, 'w').write('\n'.join(cmds) + '\n' if cmds else '0 overlay@c x %d;\n' % (x - 8))
    ring_on = '+'.join(f'between(t,{c:.3f},{c + 0.32:.3f})' for c in rings) or '0'
    graph = (f"[0]setpts=PTS-STARTPTS,fps={FPS},scale=1920:1080,sendcmd=f={cmdf}[v];"
             f"[v][2]overlay@r=x=-200:y=-200:enable='{ring_on}'[r];"
             f"[r][1]overlay@c=x={x - 8}:y={y - 6}:shortest=1,"
             f"setpts=PTS/{speed},fps={FPS},scale={W}:{H},format=yuv420p")
    ff('-ss', f'{off:.3f}', '-t', f'{b - a:.3f}', '-i', path, '-loop', '1', '-i', f'{A}/cursor.png',
       '-loop', '1', '-i', f'{A}/ring.png', '-filter_complex', graph, '-an',
       # the looping pointer pictures never end: stop at the piece's length
       '-t', f'{(b - a) / speed:.3f}', *enc(out))
    return out


def scene(name, caption, segs):
    parts = [segment(f'{name}_{j}', *s) for j, s in enumerate(segs)]
    n = len(parts)
    ins = sum((['-i', p] for p in parts), [])
    fs = int(46 * K)
    graph = (''.join(f'[{j}]' for j in range(n)) + f'concat=n={n}:v=1:a=0,'
             f"drawtext=fontfile={FONT}:text='{esc(caption)}':fontsize={fs}:fontcolor=white:"
             f"box=1:boxcolor=0x101216@0.78:boxborderw={int(24 * K)}:"
             f"x=(w-tw)/2:y=h-{int(150 * K)}+{int(40 * K)}*pow(max(0\\,1-t/0.35)\\,2):alpha='min(1\\,t/0.3)',"
             f"fade=in:d=0.25,format=yuv420p")
    open(f'{WORK}/{name}.fg', 'w').write(graph)
    ff(*ins, '-filter_complex_script', f'{WORK}/{name}.fg', *enc(f'{WORK}/{name}.mp4'))
    return f'{WORK}/{name}.mp4'


def text(t, size, color, y, start):
    return (f"drawtext=fontfile={FONT}:text='{esc(t)}':fontsize={int(size * K)}:fontcolor={color}:x=(w-tw)/2:"
            f"y={int(y * K)}+{int(30 * K)}*pow(max(0\\,1-(t-{start})/0.3)\\,2):alpha='min(1\\,max(0\\,(t-{start})/0.3))'")


def card(name, dur, body):
    open(f'{WORK}/{name}.fg', 'w').write(body)
    ff('-f', 'lavfi', '-i', f'color=c={BG}:s={W}x{H}:r={FPS}:d={dur}', '-loop', '1', '-i', f'{A}/man-logo-v2.png',
       '-filter_complex_script', f'{WORK}/{name}.fg', '-t', f'{dur}', *enc(f'{WORK}/{name}.mp4'))
    return f'{WORK}/{name}.mp4'


def logo(y, size, start=0.0):
    return (f"[1]scale={int(size * K)}:-1,format=rgba,fade=in:st={start}:d=0.4:alpha=1[l];"
            f"[0][l]overlay=x=(W-w)/2:y={int(y * K)}+{int(40 * K)}*pow(max(0\\,1-(t-{start})/0.45)\\,2):shortest=1")


def title(name, num, head, sub, dur=3.5):
    return card(name, dur, logo(250, 150) + ',' + text(f'Chapter {num}', 40, ACC, 470, 0.2) + ',' +
                text(head, 110, 'white', 530, 0.35) + ',' + text(sub, 46, '0xc0c4cc', 690, 0.6) +
                f",fade=in:d=0.3,fade=out:st={dur - 0.4}:d=0.4,format=yuv420p")


def intro():
    return card('intro', 5.5, logo(220, 300) + ',' + text('MAN 10 TEST 7', 120, 'white', 560, 0.5) + ',' +
                text('From the USB stick to every day', 54, ACC, 730, 1.0) +
                ",fade=in:d=0.4,fade=out:st=5.0:d=0.5,format=yuv420p")


def end(dur=10.0):
    warn = ['MAN is not completed yet. By downloading the TEST version you agree that the system',
            'can break or stop working and require a reinstall. Use at your own risk!']
    body = (logo(110, 220) + ',' + text('Get MAN for free', 96, 'white', 380, 0.3) + ',' +
            text('mancorporation.com', 64, ACC, 520, 0.7) + ',' +
            text('Music: ' + ', '.join(CREDITS[:3]) + ',', 26, '0x9a9ea6', 650, 1.2) + ',' +
            text(', '.join(CREDITS[3:5]) + '. ' + CREDITS[5] + '.', 26, '0x9a9ea6', 690, 1.2) + ',' +
            f"drawbox=x=0:y=ih-{int(190 * K)}:w=iw:h={int(190 * K)}:color=0x2b1d10@1:t=fill:enable='gte(t\\,1.5)'," +
            text(warn[0], 34, '0xffc56b', 935, 1.5) + ',' + text(warn[1], 34, '0xffc56b', 985, 1.5) +
            f",fade=out:st={dur - 1.5}:d=1.5,format=yuv420p")
    return card('end', dur, body)


def duration(p):
    out = subprocess.run([os.path.join(os.path.dirname(FF), 'ffprobe') if os.sep in FF else 'ffprobe', '-v', 'error', '-show_entries', 'format=duration',
                          '-of', 'csv=p=0', p], capture_output=True, text=True, check=True).stdout
    return float(out)


def build():
    chapters = []  # (chapter number, [clips])
    first = [intro()]
    for ch, items in STORY.items():
        clips = first if ch == 1 else []
        for i, it in enumerate(items):
            name = f'ch{ch}_{i:02d}'
            if it[0] == 'title':
                clips.append(title(name, *it[1:]))
            elif it[0] == 'scene':
                clips.append(scene(name, it[1], [(t, a, b, round(sp * CH_SPEED[ch], 3)) for t, a, b, sp in it[2]]))
            else:
                clips.append(end())
            print(f'{name} {duration(clips[-1]):6.1f} s  {it[1] if len(it) > 1 else "end"}', flush=True)
        chapters.append((ch, clips))
    # Video: everything in a row. Music: each chapter its song, faded, the
    # songs crossfaded over 1.5 s at the chapter changes.
    allclips = [c for _, cl in chapters for c in cl]
    with open(f'{WORK}/list.txt', 'w') as f:
        f.writelines(f"file '{p}'\n" for p in allclips)
    ff('-f', 'concat', '-safe', '0', '-i', f'{WORK}/list.txt', '-c', 'copy', f'{WORK}/video.mp4')
    lens = [sum(duration(c) for c in cl) for _, cl in chapters]
    X = 1.5
    ins, parts = [], []
    for j, ((ch, _), d) in enumerate(zip(chapters, lens)):
        ins += ['-i', os.path.join(MUSIC, SONG[ch])]
        # each song plays its chapter plus the crossfade into the next
        L = d + (X if j < len(lens) - 1 else 0)
        fade_out = f'afade=t=out:st={L - 3:.3f}:d=3' if j == len(lens) - 1 else f'afade=t=out:st={L - X:.3f}:d={X}'
        parts.append(f'[{j + 1}:a]atrim=0:{L:.3f},asetpts=PTS-STARTPTS,afade=t=in:d=0.8,{fade_out},volume=0.9[m{j}]')
    mix = '[m0]'
    for j in range(1, len(lens)):
        parts.append(f'{mix}[m{j}]acrossfade=d={X}[x{j}]')
        mix = f'[x{j}]'
    ff('-i', f'{WORK}/video.mp4', *ins, '-filter_complex', ';'.join(parts),
       '-map', '0:v', '-map', mix, '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k', '-shortest', '-movflags', '+faststart', OUT)
    total = sum(lens)
    print(f'{OUT}: {total:.1f} s ({int(total // 60)}:{int(total % 60):02d}); chapters ' +
          ', '.join(f'{d:.0f}' for d in lens) + f' s  (work files: {WORK})')


build()
