#!/usr/bin/env python3
"""make-duhach.py REC_DIR MUSIC.mp3 OUT.mp4 — the DUHACH! video, edited to the music.

Takes are Windows-VM recordings (1280x800, 16:10) made with director.py
(f*.ppm + cursor.json) or plain screen grabs (g*.ppm, no cursor) of the
restarts. They are shown 16:9 over a blurred copy of themselves. Cuts land
on bar lines of "TUDU DYŠ!!!!! DRUM A BASS" (174 BPM). Needs ffmpeg (FFMPEG=…).
"""
import json, os, subprocess, sys, tempfile

REC, MUSIC, OUT = sys.argv[1:4]
FF = os.environ.get('FFMPEG', 'ffmpeg')
HERE = os.path.dirname(os.path.abspath(__file__))
A = os.path.join(HERE, 'assets')
FONT = '/usr/share/fonts/opentype/cantarell/Cantarell-VF.otf'
FPS = 30
BPM, FIRST = 174.0, 0.0
BAR = 4 * 60 / BPM
BG, ACC, ORANGE = '0x15171c', '0x3584e4', '0xff7800'
W = tempfile.mkdtemp(prefix='duhach-video-')

# (kind, bars, caption, [(take, from, to)]) — from/to in seconds of the take.
STORY = [
    ('intro', 2),
    ('scene', 3, 'Install MAN right from Windows', [('dw1', 0.5, 3.6), ('dw1', 9.6, 11.5), ('dw1', 27.0, 34.0)]),
    ('scene', 3, 'Pick the MAN file you downloaded', [('dw2', 14.6, 18.0), ('dw2', 20.8, 25.0), ('dw2', 45.5, 47.5)]),
    ('scene', 3, 'Choose how much space MAN gets', [('dw2', 47.5, 54.5)]),
    ('scene', 2, 'One click to prepare', [('dw2', 54.5, 63.0)]),
    ('scene', 3, 'Windows makes room. Your files stay.', [('dw2', 63.0, 152.0)]),
    ('scene', 2, 'Restart…', [('dw4', 0.4, 4.4), ('dw5', 0.0, 5.0)]),
    ('scene', 2, '…straight into the MAN installer', [('dw5', 29.0, 34.0)]),
    ('scene', 3, 'Install Alongside Windows', [('dw6', 0.3, 12.5)]),
    ('scene', 3, 'MAN installs next to Windows', [('dw6', 12.5, 109.0)]),
    ('scene', 2, 'Done!', [('dw6', 109.0, 113.0), ('dw7', 0.3, 2.0)]),
    ('scene', 3, 'Choose MAN or Windows at every start', [('dw8', 3.0, 7.6), ('dw8', 34.0, 44.5)]),
    ('end', 8),
]


def ff(*args):
    subprocess.run([FF, '-v', 'error', '-y', *args], check=True)


def enc(path):
    return ['-r', str(FPS), '-c:v', 'libx264', '-preset', 'medium', '-crf', '17', '-pix_fmt', 'yuv420p', path]


def esc(text):
    return text.replace('\\', '\\\\').replace("'", "’").replace(':', '\\:').replace(',', '\\,')


def take_clip(take):
    """A take as a real-time 30 fps 1280x800 clip, with cursor and click rings when logged."""
    out = f'{W}/raw_{take}.mp4'
    if os.path.exists(out):
        return out
    d = os.path.join(REC, take)
    times = [float(x) for x in open(f'{d}/times.txt').read().split()]
    rate = len(times) / max(times[-1], 0.1)
    pre = 'f' if os.path.exists(f'{d}/f00000.ppm') else 'g'
    src = ['-framerate', f'{rate:.4f}', '-i', f'{d}/{pre}%05d.ppm']
    if not os.path.exists(f'{d}/cursor.json'):
        ff(*src, '-vf', f'fps={FPS},scale=1280:800,format=yuv420p', *enc(out))
        return out
    log = json.load(open(f'{d}/cursor.json'))
    cmds, clicks, last = [], [], None
    for e in log:
        if (e['x'], e['y']) != last:
            cmds.append(f"{e['t']:.3f} overlay@c x {e['x'] - 5}, overlay@c y {e['y'] - 4};")
            last = (e['x'], e['y'])
        if e['k'] == 'click':
            clicks.append(e['t'])
            cmds.append(f"{e['t']:.3f} overlay@r x {e['x'] - 40}, overlay@r y {e['y'] - 40};")
    open(f'{W}/{take}.cmd', 'w').write('\n'.join(cmds) + '\n')
    ring_on = '+'.join(f'between(t,{c:.3f},{c + 0.32:.3f})' for c in clicks) or '0'
    graph = (f"[0]fps={FPS},scale=1280:800,sendcmd=f={W}/{take}.cmd[v];"
             f"[v][2]overlay@r=x=-200:y=-200:shortest=1:enable='{ring_on}'[r];"
             f"[r][1]overlay@c=x={log[0]['x'] - 5}:y={log[0]['y'] - 4}:shortest=1,format=yuv420p")
    ff(*src, '-loop', '1', '-i', f'{A}/cursor-small.png', '-loop', '1', '-i', f'{A}/ring-small.png',
       '-filter_complex', graph, *enc(out))
    return out


def scene(name, frames, caption, segs):
    dur = frames / FPS
    speed = sum(b - a for _, a, b in segs) / dur
    ins, parts = [], []
    for i, (take, a, b) in enumerate(segs):
        ins += ['-i', take_clip(take)]
        parts.append(f"[{i}]trim={a}:{b},setpts=(PTS-STARTPTS)/{speed:.4f}[p{i}]")
    n = len(segs)
    graph = ';'.join(parts) + ';' + ''.join(f'[p{i}]' for i in range(n)) + f"concat=n={n}:v=1:a=0,fps={FPS},split[a][b];" + (
        # 16:10 picture on a blurred, darkened copy of itself
        "[a]scale=1920:1200,crop=1920:1080,boxblur=30:3,eq=brightness=-0.18[bg];"
        "[b]scale=1680:1050[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,"
        f"scale=2112:1188,zoompan=z='1+0.07*exp(-on/5)+0.03*on/{frames}':x='iw/2-iw/zoom/2':y='ih/2-ih/zoom/2':d=1:s=1920x1080:fps={FPS},"
        f"drawtext=fontfile={FONT}:text='{esc(caption)}':fontsize=58:fontcolor=white:box=1:boxcolor=0x0f1115@0.78:boxborderw=28:"
        f"x=(w-tw)/2:y=h-170+60*pow(max(0\\,1-t/0.3)\\,2):alpha='min(1\\,t/0.2)',"
        f"tpad=stop_mode=clone:stop_duration=2,trim=duration={dur:.4f},format=yuv420p")
    open(f'{W}/{name}.fg', 'w').write(graph)
    ff(*ins, '-filter_complex_script', f'{W}/{name}.fg', *enc(f'{W}/{name}.mp4'))


def text(t, size, color, y, start, extra=''):
    return (f"drawtext=fontfile={FONT}:text='{esc(t)}':fontsize={size}:fontcolor={color}:x=(w-tw)/2:"
            f"y={y}+30*pow(max(0\\,1-(t-{start})/0.3)\\,2):alpha='min(1\\,max(0\\,(t-{start})/0.25))'{extra}")


def card(name, frames, body, logo_file):
    dur = frames / FPS
    open(f'{W}/{name}.fg', 'w').write(body)
    ff('-f', 'lavfi', '-i', f'color=c={BG}:s=1920x1080:r={FPS}:d={dur:.4f}', '-loop', '1', '-i', logo_file,
       '-filter_complex_script', f'{W}/{name}.fg', '-t', f'{dur:.4f}', *enc(f'{W}/{name}.mp4'))


def logo(y, size, start=0.0):
    return (f"[1]scale={size}:-1,format=rgba,fade=in:st={start}:d=0.3:alpha=1[l];"
            f"[0][l]overlay=x=(W-w)/2:y={y}+50*pow(max(0\\,1-(t-{start})/0.35)\\,2):shortest=1")


def build():
    t, files = 0.0, []
    for i, item in enumerate(STORY):
        kind = item[0]
        end = t + item[1] * BAR
        frames = round(end * FPS) - round(t * FPS)
        name = f'part{i:02d}'
        if kind == 'intro':
            card(name, frames, logo(250, 280) + ',' + text('DUHACH!', 150, 'white', 590, 0.2) + ',' +
                 text('Install MAN. No USB stick needed.', 54, ORANGE, 780, 0.5) + ",fade=in:d=0.2,format=yuv420p",
                 f'{A}/duhach.png')
        elif kind == 'scene':
            scene(name, frames, item[2], item[3])
        elif kind == 'end':
            warn = ['MAN is not completed yet. By downloading the TEST version you agree that the system',
                    'can break or stop working and require a reinstall. Use at your own risk!']
            body = logo(130, 240) + ',' + text('DUHACH!', 150, 'white', 400, 0.2) + ',' + \
                text('MAN 10 TEST, right from Windows', 66, ORANGE, 600, 0.2 + BAR) + ',' + \
                text('github.com/MANCorporation/MAN', 44, '0xc0c4cc', 710, 0.2 + 2 * BAR) + ',' + \
                "drawbox=x=0:y=ih-190:w=iw:h=190:color=0x2b1d10@1:t=fill:enable='gte(t\\,1.5)'," + \
                text(warn[0], 36, '0xffc56b', 935, 1.5) + ',' + text(warn[1], 36, '0xffc56b', 990, 1.5) + \
                f",fade=out:st={frames / FPS - 1.5:.3f}:d=1.5,format=yuv420p"
            card(name, frames, body, f'{A}/duhach.png')
        files.append(f'{W}/{name}.mp4')
        print(f'{name} {kind:6} {t:6.2f} → {end:6.2f}', flush=True)
        t = end
    with open(f'{W}/list.txt', 'w') as f:
        f.writelines(f"file '{p}'\n" for p in files)
    ff('-f', 'concat', '-safe', '0', '-i', f'{W}/list.txt', '-c', 'copy', f'{W}/video.mp4')
    ff('-i', f'{W}/video.mp4', '-i', MUSIC, '-filter_complex',
       f'[1:a]atrim=0:{t:.3f},afade=t=in:d=0.3,afade=t=out:st={t - 3:.3f}:d=3[m]',
       '-map', '0:v', '-map', '[m]', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '192k', '-movflags', '+faststart', OUT)
    print(f'{OUT}: {t:.2f} s  (work files: {W})')


build()
