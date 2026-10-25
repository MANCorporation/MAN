#!/usr/bin/env python3
"""make-intro2.py REC_DIR MUSIC.mp3 OUT.mp4 — the MAN 10 intro, edited to the music.

REC_DIR holds takes filmed with director.py (PPM frames, times.txt, cursor.json).
Every cut lands on a bar line of the song (115.4 BPM, first beat at 0.48 s); a
cursor and click ripples are drawn from cursor.json (QEMU's screendump has no
pointer). Needs ffmpeg (FFMPEG=…), assets/ next to this script.
"""
import json, os, subprocess, sys, tempfile

REC, MUSIC, OUT = sys.argv[1:4]
FF = os.environ.get('FFMPEG', 'ffmpeg')
HERE = os.path.dirname(os.path.abspath(__file__))
A = os.path.join(HERE, 'assets')
FONT = '/usr/share/fonts/opentype/cantarell/Cantarell-VF.otf'
FPS = 30
BPM, FIRST = 115.4, 0.48
BAR = 4 * 60 / BPM
BG, ACC = '0x15171c', '0x3584e4'
W = tempfile.mkdtemp(prefix='man-intro2-')

# ── storyboard: (kind, bars, …) ─────────────────────────────────────────
# take segments are (take, from, to) in seconds of the recording; each
# scene's segments are sped up together to fill its bars.
STORY = [
    ('intro', None),
    ('scene', 3, 'Everything in one place', [('nA', 1.6, 11.2)]),
    ('scene', 4, 'Find an app. Click Install.', [('sF1', 1.8, 5.6), ('sF1', 6.2, 10.6), ('sF1', 15.2, 17.6), ('sF1', 19.6, 21.2), ('sF1', 28.6, 30.6)]),
    ('scene', 2, 'Games…', [('sF3', 1.0, 3.4), ('sF3', 6.0, 6.8), ('sF3', 11.7, 13.4), ('sF3', 15.3, 17.0)]),
    ('scene', 2, '…and so much more', [('sF2', 1.0, 3.4), ('sF2', 6.0, 6.8), ('sF2', 10.2, 12.2), ('sF2', 14.8, 16.0), ('sF2', 23.0, 24.2)]),
    ('count', 2),
    ('scene', 3, 'Updates in a few clicks', [('nB', 0.3, 11.4)]),
    ('scene', 3, 'Go back in time, anytime', [('nC', 0.3, 7.8), ('nC2', 0.0, 5.9)]),
    ('scene', 2, 'Install any app file, step by step', [('nD', 0.3, 7.1)]),
    ('scene', 2, 'Your music, ready to play', [('nE', 0.3, 8.8)]),
    ('end', 5),
]


def ff(*args):
    subprocess.run([FF, '-v', 'error', '-y', *args], check=True)


def enc(path):
    return ['-r', str(FPS), '-c:v', 'libx264', '-preset', 'medium', '-crf', '17', '-pix_fmt', 'yuv420p', path]


def esc(text):  # drawtext text= value inside a filter script
    return text.replace('\\', '\\\\').replace("'", "’").replace(':', '\\:').replace(',', '\\,')


# ── 1. takes with cursor: real-time 30 fps clips ───────────────────────
def cursor_clip(take):
    out = f'{W}/raw_{take}.mp4'
    if os.path.exists(out):
        return out
    d = os.path.join(REC, take)
    log = json.load(open(f'{d}/cursor.json'))
    cmds, clicks, last = [], [], None
    for e in log:
        if (e['x'], e['y']) != last:
            cmds.append(f"{e['t']:.3f} overlay@c x {e['x'] - 8}, overlay@c y {e['y'] - 6};")
            last = (e['x'], e['y'])
        if e['k'] == 'click':
            clicks.append(e['t'])
            cmds.append(f"{e['t']:.3f} overlay@r x {e['x'] - 60}, overlay@r y {e['y'] - 60};")
    open(f'{W}/{take}.cmd', 'w').write('\n'.join(cmds) + '\n')
    ring_on = '+'.join(f'between(t,{c:.3f},{c + 0.32:.3f})' for c in clicks) or '0'
    x0, y0 = log[0]['x'] - 8, log[0]['y'] - 6
    graph = (f"[0]fps={FPS},sendcmd=f={W}/{take}.cmd[v];"
             f"[v][2]overlay@r=x=-200:y=-200:shortest=1:enable='{ring_on}'[r];"
             f"[r][1]overlay@c=x={x0}:y={y0}:shortest=1,format=yuv420p")
    ff('-framerate', '15', '-i', f'{d}/f%05d.ppm', '-loop', '1', '-i', f'{A}/cursor.png', '-loop', '1', '-i', f'{A}/ring.png',
       '-filter_complex', graph, *enc(out))
    return out


# ── 2. scenes ──────────────────────────────────────────────────────────
def scene(name, frames, caption, segs):
    dur = frames / FPS
    src = sum(b - a for _, a, b in segs)
    speed = src / dur
    ins, parts = [], []
    for i, (take, a, b) in enumerate(segs):
        ins += ['-i', cursor_clip(take)]
        parts.append(f"[{i}]trim={a}:{b},setpts=(PTS-STARTPTS)/{speed:.4f}[p{i}]")
    n = len(segs)
    cap = esc(caption)
    graph = ';'.join(parts) + ';' + ''.join(f'[p{i}]' for i in range(n)) + f"concat=n={n}:v=1:a=0,fps={FPS}," + (
        # punch-in on the cut, then a slow push
        f"scale=2112:1188,zoompan=z='1+0.07*exp(-on/5)+0.03*on/{frames}':x='iw/2-iw/zoom/2':y='ih/2-ih/zoom/2':d=1:s=1920x1080:fps={FPS},"
        f"drawtext=fontfile={FONT}:text='{cap}':fontsize=58:fontcolor=white:box=1:boxcolor=0x0f1115@0.72:boxborderw=28:"
        f"x=(w-tw)/2:y=h-170+60*pow(max(0\\,1-t/0.35)\\,2):alpha='min(1\\,t/0.25)',"
        f"tpad=stop_mode=clone:stop_duration=2,trim=duration={dur:.4f},format=yuv420p")
    open(f'{W}/{name}.fg', 'w').write(graph)
    ff(*ins, '-filter_complex_script', f'{W}/{name}.fg', *enc(f'{W}/{name}.mp4'))


def text(t, size, color, y, start, extra=''):
    return (f"drawtext=fontfile={FONT}:text='{esc(t)}':fontsize={size}:fontcolor={color}:x=(w-tw)/2:"
            f"y={y}+30*pow(max(0\\,1-(t-{start})/0.3)\\,2):alpha='min(1\\,max(0\\,(t-{start})/0.25))'{extra}")


def card(name, frames, body):
    dur = frames / FPS
    open(f'{W}/{name}.fg', 'w').write(body)
    ff('-f', 'lavfi', '-i', f'color=c={BG}:s=1920x1080:r={FPS}:d={dur:.4f}', '-loop', '1', '-i', f'{A}/logo.png',
       '-filter_complex_script', f'{W}/{name}.fg', '-t', f'{dur:.4f}', *enc(f'{W}/{name}.mp4'))


def logo(y, size, start=0.0):
    # slides up and fades in
    return (f"[1]scale={size}:-1,format=rgba,fade=in:st={start}:d=0.35:alpha=1[l];"
            f"[0][l]overlay=x=(W-w)/2:y={y}+50*pow(max(0\\,1-(t-{start})/0.4)\\,2):shortest=1")


def build():
    t = 0.0
    files = []
    for i, item in enumerate(STORY):
        kind = item[0]
        end = FIRST + BAR if kind == 'intro' else t + item[1] * BAR
        frames = round(end * FPS) - round(t * FPS)
        name = f'part{i:02d}'
        if kind == 'intro':
            card(name, frames, logo(300, 260) + ',' + text('MAN', 150, 'white', 590, 0.25) + ',' +
                 text('Simple. Friendly. Yours.', 54, ACC, 770, 0.6) + ",fade=in:d=0.25,format=yuv420p")
        elif kind == 'scene':
            scene(name, frames, item[2], item[3])
        elif kind == 'count':
            # 0 → 3,300 counting up, ease-out
            v = 'floor(3300*(1-pow(1-min(1,max(0,(t-0.35)/1.3)),3)))'
            open(f'{W}/count.txt', 'w').write(f'%{{eif:floor({v}/1000):d}},%{{eif:mod({v},1000):d:3}}+')
            card(name, frames,
                 "[0]drawbox=x=0:y=0:w=iw:h=ih:color=0x1a2a44@0.35:t=fill[b];[b]null," +
                 text('Install apps with one click', 70, 'white', 250, 0.0) + ',' +
                 f"drawtext=fontfile={FONT}:textfile={W}/count.txt:fontsize=300:fontcolor={ACC}:x=(w-tw)/2:y=390:"
                 f"alpha='min(1\\,max(0\\,(t-0.25)/0.2))'," +
                 text('apps on Flathub', 64, '0xc0c4cc', 760, 1.6) + ',format=yuv420p')
        elif kind == 'end':
            warn = ['MAN is not completed yet. By downloading the TEST version you agree that the system',
                    'can break or stop working and require a reinstall. Use at your own risk!']
            body = logo(160, 240) + ',' + text('MAN 10', 170, 'white', 430, 0.3) + ',' + \
                text('Try the TEST version', 72, ACC, 650, 0.3 + BAR / 2) + ',' + \
                "drawbox=x=0:y=ih-190:w=iw:h=190:color=0x2b1d10@1:t=fill:enable='gte(t\\,1.5)'," + \
                text(warn[0], 36, '0xffc56b', 935, 1.5) + ',' + text(warn[1], 36, '0xffc56b', 990, 1.5) + \
                f",fade=out:st={frames / FPS - 1.2:.3f}:d=1.2,format=yuv420p"
            card(name, frames, body)
        files.append(f'{W}/{name}.mp4')
        print(f'{name} {kind:6} {t:6.2f} → {end:6.2f}', flush=True)
        t = end
    with open(f'{W}/list.txt', 'w') as f:
        f.writelines(f"file '{p}'\n" for p in files)
    ff('-f', 'concat', '-safe', '0', '-i', f'{W}/list.txt', '-c', 'copy', f'{W}/video.mp4')
    ff('-i', f'{W}/video.mp4', '-i', MUSIC, '-filter_complex',
       f'[1:a]atrim=0:{t:.3f},afade=t=in:d=0.4,afade=t=out:st={t - 2.5:.3f}:d=2.5[m]',
       '-map', '0:v', '-map', '[m]', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k', '-movflags', '+faststart', OUT)
    print(f'{OUT}: {t:.2f} s  (work files: {W})')


build()
