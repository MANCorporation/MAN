#!/usr/bin/env python3
"""MAN boot menu icons (own drawings in the Adwaita style), written as SVG and
rendered to PNG for GRUB.

    dev-tools/bootmenu/make-icons.py OUT_DIR

OUT_DIR gets svg/<name>.svg and <name>.png (256 px; GRUB scales them down).
Names are GRUB menu classes: an entry with --class windows11 shows windows11.png.
The Windows and macOS emblems are simple drawings that only name the system
(no copied artwork); macOS versions get an abstract colour tile each.
"""
import os
import subprocess
import sys

RSVG = os.path.expanduser("~/Documents/MANRUST/tools/rsvg/rsvg-convert")



def svg(body, defs=""):
    return ('<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" '
            f'viewBox="0 0 128 128"><defs>{defs}</defs>{body}</svg>')


def lin(id_, stops, x1=0, y1=0, x2=0, y2=1):
    s = "".join(f'<stop offset="{o}" stop-color="{c}"/>' for o, c in stops)
    return f'<linearGradient id="{id_}" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">{s}</linearGradient>'


SHADOW = ('<filter id="sh" x="-20%" y="-20%" width="140%" height="140%">'
          '<feGaussianBlur stdDeviation="2.2"/></filter>')


# ── the drive: a front-facing disk, light body, darker base, slot and light ──
def drive(emblem, emblem_defs="", led="#33d17a"):
    defs = SHADOW + lin("body", [(0, "#ffffff"), (1, "#deddda")]) \
        + lin("base", [(0, "#c0bfbc"), (1, "#9a9996")]) \
        + lin("front", [(0, "#f6f5f4"), (1, "#d5d3cf")]) + emblem_defs
    body = (
        '<rect x="12" y="24" width="104" height="94" rx="12" fill="#000" opacity=".45" filter="url(#sh)"/>'
        '<rect x="12" y="22" width="104" height="94" rx="12" fill="url(#base)"/>'
        '<rect x="12" y="16" width="104" height="94" rx="12" fill="url(#body)"/>'
        '<rect x="12.5" y="16.5" width="103" height="93" rx="11.5" fill="none" stroke="#fff" stroke-opacity=".8"/>'
        '<path d="M12 88 H116 V98 a12 12 0 0 1 -12 12 H24 a12 12 0 0 1 -12 -12 Z" fill="url(#front)"/>'
        '<path d="M12 88.5 H116" stroke="#9a9996" stroke-opacity=".6"/>'
        '<rect x="30" y="97" width="52" height="4" rx="2" fill="#5e5c64"/>'
        f'<circle cx="97" cy="99" r="3" fill="{led}"/>'
        '<circle cx="21" cy="24" r="2" fill="#c0bfbc"/><circle cx="107" cy="24" r="2" fill="#c0bfbc"/>'
        f'<g transform="translate(64 52)">{emblem}</g>'
    )
    return svg(body, defs)


# ── the USB stick: coloured body with the emblem, metal plug below ──
def stick(c1, c2, emblem, emblem_defs=""):
    defs = SHADOW + lin("sb", [(0, c1), (1, c2)]) \
        + lin("plug", [(0, "#f6f5f4"), (1, "#c0bfbc")]) + emblem_defs
    body = (
        '<rect x="34" y="14" width="60" height="84" rx="16" fill="#000" opacity=".45" filter="url(#sh)"/>'
        '<rect x="46" y="84" width="36" height="34" rx="4" fill="url(#plug)"/>'
        '<rect x="46.5" y="84.5" width="35" height="33" rx="3.5" fill="none" stroke="#9a9996"/>'
        '<rect x="54" y="100" width="7" height="7" rx="1" fill="#77767b"/><rect x="67" y="100" width="7" height="7" rx="1" fill="#77767b"/>'
        '<rect x="34" y="10" width="60" height="84" rx="16" fill="url(#sb)"/>'
        '<rect x="34" y="88" width="60" height="6" fill="#000" opacity=".12"/>'
        '<rect x="34.5" y="10.5" width="59" height="83" rx="15.5" fill="none" stroke="#fff" stroke-opacity=".25"/>'
        f'<g transform="translate(64 50) scale(.78)">{emblem}</g>'
    )
    return svg(body, defs)


# ── emblems, drawn around (0,0), about 56 px wide ──
# The MAN logo (version 2, man-installer/data/logo/man-logo-v2.svg).
LOGO_PATH = "M653 642 H3443 L4096 3268 L3348 3453 L2838 1414 H1257 L748 3453 L0 3268 Z"
LOGO_GREEN = "#26ea75"


def logo(width, cx=0, cy=0, color=LOGO_GREEN):
    s = width / 4096
    h = 2811 * s
    return (f'<g transform="translate({cx - width / 2} {cy - h / 2 - 642 * s}) scale({s})">'
            f'<path d="{LOGO_PATH}" fill="{color}"/></g>')


def man_badge(size=54):
    # A dark rounded badge with the green logo.
    s = size / 54
    return (f'<g transform="scale({s})">'
            '<rect x="-27" y="-24" width="54" height="54" rx="13" fill="#000" opacity=".35"/>'
            '<rect x="-27" y="-27" width="54" height="54" rx="13" fill="url(#mb)"/>'
            '<rect x="-26.5" y="-26.5" width="53" height="53" rx="12.5" fill="none" stroke="#fff" stroke-opacity=".12"/>'
            + logo(36) + '</g>')


MAN_DEFS = lin("mb", [(0, "#3d3846"), (1, "#1e1e20")])


def m_white(size=46, width=13):
    return logo(size)


def recovery_badge():
    # The same badge, a white circular arrow around a small logo: "start over / repair".
    return ('<rect x="-27" y="-24" width="54" height="54" rx="13" fill="#000" opacity=".35"/>'
            '<rect x="-27" y="-27" width="54" height="54" rx="13" fill="url(#rb)"/>'
            '<path d="M -13.5 -9 A 16 16 0 1 1 -16 4" fill="none" stroke="#fff" stroke-width="4.5" stroke-linecap="round"/>'
            '<path d="M -21 -14 L -12 -2 L -6 -15 Z" fill="#fff"/>'
            + logo(15))


REC_DEFS = lin("rb", [(0, "#3d3846"), (1, "#1e1e20")])


def win11(c1="#4cc2ff", c2="#0067c0"):
    g = lin("w11", [(0, c1), (1, c2)], 0, 0, 1, 1)
    sq = "".join(f'<rect x="{x}" y="{y}" width="23" height="23" rx="1.5" fill="url(#w11)"/>'
                 for x in (-24, 1) for y in (-24, 1))
    return sq, g


def win10(color="#0078d4", light="#3aa0f3"):
    # Window seen at an angle: the left side lower than the right.
    g = lin("w10", [(0, light), (1, color)], 0, 0, 1, 1)
    panes = ('<path d="M-26 -18 L-3 -21.5 V-1.5 H-26 Z" fill="url(#w10)"/>'
             '<path d="M0 -22 L27 -26 V-1.5 H0 Z" fill="url(#w10)"/>'
             '<path d="M-26 1.5 H-3 V21.5 L-26 18 Z" fill="url(#w10)"/>'
             '<path d="M0 1.5 H27 V26 L0 22 Z" fill="url(#w10)"/>')
    return panes, g


def flag(orb=False):
    # A waving four-colour flag (Windows XP, Vista and 7 era).
    def pane(dx, dy, c):
        x, y = dx, dy
        return (f'<path d="M{x} {y+3} q6 -4 12 -1 t11 1 v20 q-6 -3 -11 -1 t-12 1 Z" '
                f'fill="{c}"/>')
    p = (pane(-24, -24, "#f25022") + pane(1, -22, "#7fba00")
         + pane(-24, 1, "#00a4ef") + pane(1, 3, "#ffb900"))
    p = f'<g transform="skewY(-4)">{p}</g>'
    if not orb:
        return p, ""
    d = ('<radialGradient id="orb" cx=".5" cy=".35" r=".7">'
         '<stop offset="0" stop-color="#3584e4"/><stop offset=".6" stop-color="#1a3d6e"/>'
         '<stop offset="1" stop-color="#0b1a33"/></radialGradient>'
         + lin("gloss", [(0, "#ffffff"), (1, "#ffffff")]))
    o = ('<circle r="31" fill="url(#orb)"/><circle r="30.5" fill="none" stroke="#99c1f1" stroke-opacity=".6"/>'
         f'<g transform="scale(.72)">{p}</g>'
         '<ellipse cy="-15" rx="22" ry="12" fill="#fff" opacity=".18"/>')
    return o, d


def mac_tile(stops, wave, name):
    # Abstract colour tile for a macOS version (no Apple artwork).
    g = (f'<linearGradient id="mt{name}" x1="0" y1="0" x2="1" y2="1">'
         + "".join(f'<stop offset="{o}" stop-color="{c}"/>' for o, c in stops)
         + '</linearGradient>'
         f'<clipPath id="mc{name}"><rect x="-28" y="-28" width="56" height="56" rx="13"/></clipPath>')
    w1, w2 = wave
    t = (f'<rect x="-28" y="-25" width="56" height="56" rx="13" fill="#000" opacity=".25"/>'
         f'<g clip-path="url(#mc{name})"><rect x="-28" y="-28" width="56" height="56" fill="url(#mt{name})"/>'
         f'<path d="M-30 6 C-14 -6 2 18 30 0 V30 H-30 Z" fill="{w1}" opacity=".75"/>'
         f'<path d="M-30 16 C-10 6 10 26 30 12 V30 H-30 Z" fill="{w2}" opacity=".8"/>'
         '<rect x="-28" y="-28" width="56" height="20" fill="#fff" opacity=".10"/></g>'
         '<rect x="-27.5" y="-27.5" width="55" height="55" rx="12.5" fill="none" stroke="#fff" stroke-opacity=".35"/>')
    return t, g


MACOS = {
    # class: (gradient stops, two wave colours)
    "macos-golden-gate": ([(0, "#ffb347"), (.5, "#f66151"), (1, "#a51d2d")], ("#e66100", "#5e1a1a")),
    "macos-tahoe": ([(0, "#99c1f1"), (.5, "#3584e4"), (1, "#1a5fb4")], ("#62a0ea", "#0b3b6e")),
    "macos-sequoia": ([(0, "#62a0ea"), (.5, "#613583"), (1, "#241f31")], ("#2ec27e", "#1c4f3a")),
    "macos-sonoma": ([(0, "#f9f06b"), (.5, "#8ff0a4"), (1, "#26a269")], ("#33d17a", "#1f6e45")),
    "macos-ventura": ([(0, "#ffbe6f"), (.5, "#ff7800"), (1, "#c64600")], ("#e01b24", "#7a1b0c")),
    "macos-monterey": ([(0, "#dc8add"), (.5, "#9141ac"), (1, "#3d3283")], ("#c061cb", "#26205e")),
    "macos-big-sur": ([(0, "#ffa8c5"), (.5, "#c061cb"), (1, "#1c71d8")], ("#f66151", "#1a5fb4")),
    "macos": ([(0, "#deddda"), (.5, "#9a9996"), (1, "#5e5c64")], ("#c0bfbc", "#3d3846")),
}


def usb_symbol():
    return ('<g fill="none" stroke="#fff" stroke-width="4.5" stroke-linecap="round" stroke-linejoin="round">'
            '<path d="M0 26 V-22"/><path d="M0 12 L-14 0 V-8"/><path d="M0 4 L13 -6 V-12"/></g>'
            '<path d="M0 -32 L-7 -21 H7 Z" fill="#fff"/><circle cx="-14" cy="-12" r="4.5" fill="#fff"/>'
            '<rect x="9" y="-18" width="8" height="8" fill="#fff"/><circle cy="27" r="5.5" fill="#fff"/>')


def marker():
    # Below the selected title: a ring with an arrow ("start this").
    return svg('<circle cx="64" cy="64" r="56" fill="#000" fill-opacity=".25"/>'
               '<circle cx="64" cy="64" r="54" fill="none" stroke="#c0bfbc" stroke-width="8"/>'
               '<path d="M64 96 V38 M40 60 L64 34 L88 60" fill="none" stroke="#c0bfbc" '
               'stroke-width="12" stroke-linecap="round" stroke-linejoin="round"/>')


def small(body, defs=""):
    return svg(body, defs)


def icons():
    out = {}
    out["man"] = drive(man_badge(), MAN_DEFS)
    out["man-recovery"] = drive(recovery_badge(), REC_DEFS, led="#ffa348")
    w, d = win11(); out["windows11"] = drive(w, d)
    w, d = win10(); out["windows10"] = drive(w, d)
    w, d = win10("#1c9ad6", "#62cff4"); out["windows8"] = drive(w, d.replace('w10', 'w10'))
    w, d = flag(); out["windows7"] = drive(w, d)
    out["windowsxp"] = out["windows7"]
    w, d = flag(orb=True); out["windowsvista"] = drive(w, d)
    w, d = win11("#c0bfbc", "#77767b"); out["windows"] = drive(w, d)
    for name, (stops, wave) in MACOS.items():
        t, g = mac_tile(stops, wave, name.replace("-", ""))
        out[name] = drive(t, g)
    out["os"] = drive('<circle r="22" fill="none" stroke="#9a9996" stroke-width="5"/><circle r="6" fill="#9a9996"/>')
    out["usb"] = stick("#ffa348", "#ff7800", usb_symbol())
    out["usb-man"] = stick("#3d3846", "#1e1e20", logo(52))
    w, d = win11("#ffffff", "#deddda"); out["usb-windows"] = stick("#5e5c64", "#3d3846", w, d)
    t, g = mac_tile(*MACOS["macos"], "usb"); out["usb-macos"] = stick("#deddda", "#9a9996", t, g)
    out["marker"] = marker()
    # Small icons for the Advanced Options list.
    out["firmware"] = small(
        '<rect x="28" y="28" width="72" height="72" rx="10" fill="#5e5c64"/>'
        '<rect x="40" y="40" width="48" height="48" rx="6" fill="#9a9996"/>'
        + "".join(f'<rect x="{x}" y="14" width="8" height="14" rx="2" fill="#c0bfbc"/>'
                  f'<rect x="{x}" y="100" width="8" height="14" rx="2" fill="#c0bfbc"/>'
                  f'<rect x="14" y="{x}" width="14" height="8" rx="2" fill="#c0bfbc"/>'
                  f'<rect x="100" y="{x}" width="14" height="8" rx="2" fill="#c0bfbc"/>' for x in (40, 60, 80)))
    out["snapshot"] = small(
        '<circle cx="64" cy="64" r="48" fill="none" stroke="#c0bfbc" stroke-width="10"/>'
        '<path d="M64 36 V64 L84 76" fill="none" stroke="#c0bfbc" stroke-width="10" stroke-linecap="round"/>'
        '<path d="M8 50 L24 74 L40 50 Z" fill="#c0bfbc"/>')
    out["update"] = small(
        '<circle cx="64" cy="64" r="52" fill="#3584e4"/>'
        '<path d="M64 32 V86 M40 64 L64 90 L88 64" fill="none" stroke="#fff" stroke-width="12" '
        'stroke-linecap="round" stroke-linejoin="round"/>')
    out["gnu-linux"] = out["linux"] = svg(f'<g transform="translate(64 64)">{man_badge(108)}</g>', MAN_DEFS)
    out["back"] = small('<path d="M76 30 L42 64 L76 98" fill="none" stroke="#c0bfbc" stroke-width="12" '
                        'stroke-linecap="round" stroke-linejoin="round"/>')
    return out


def main():
    out_dir = sys.argv[1]
    os.makedirs(os.path.join(out_dir, "svg"), exist_ok=True)
    for name, text in icons().items():
        s = os.path.join(out_dir, "svg", name + ".svg")
        with open(s, "w") as f:
            f.write(text)
        size = "256"
        subprocess.run([RSVG, "-w", size, "-h", size, s, "-o", os.path.join(out_dir, name + ".png")], check=True)
    print(f"{len(icons())} icons in {out_dir}")


if __name__ == "__main__":
    main()
