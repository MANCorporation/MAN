#!/usr/bin/env python3
"""Restyle the MAN website in MAN's own look (logo v2, boot screens, Adwaita dark):
black and Adwaita dark greys instead of navy, the logo's green instead of
violet / sky blue / pink, no galaxy picture behind the top of the page.

    man-style.py SITE_DIR

Rewrites SITE_DIR/css/style.css from the colours below (run it on the
original style.css, e.g. from the 2026-11-19 backup) and writes
css/man-style.css with the few rules a colour swap cannot do (dark text on
green buttons). The pages load man-style.css after style.css.
"""
import re
import sys

site = sys.argv[1]
GREEN = (38, 234, 117)       # #26ea75, the logo
GREEN_LIGHT = (92, 240, 154)  # #5cf09a
GREEN_DEEP = (24, 184, 92)    # #18b85c
TEAL = (45, 212, 163)         # #2dd4a3

hex_map = {
    "#7c5cff": "#26ea75",   # violet: buttons, accents
    "#5835e5": "#18b85c",   # deep violet
    "#a78bfa": "#5cf09a",   # light violet
    "#c4b5fd": "#b8f7d0",   # very light violet (text gradients)
    "#e9d5ff": "#d9fbe6",
    "#c04bd8": "#2dd4a3",   # pink
    "#38bdf8": "#2dd4a3",   # sky blue
    "#7dd3fc": "#9ff2c9",
    "#93c5fd": "#9ff2c9",
    "#3584e4": "#26ea75",
    "#cbd5e1": "#c0bfbc",   # slate text -> Adwaita light 4
    "#e2e8f0": "#deddda",
    "#94a3b8": "#9a9996",
    "#64748b": "#77767b",
    "#f1f5f9": "#f6f5f4",
    "#f8fafc": "#fafafa",
    "#070814": "#000000",   # page background
    "#05060f": "#000000",
    "#090a18": "#0a0a0b",
    "#1a1622": "#1e1e20",
}
rgb_map = {
    (124, 92, 255): GREEN,
    (167, 139, 250): GREEN_LIGHT,
    (56, 189, 248): TEAL,
    (192, 75, 216): GREEN_DEEP,
    (16, 20, 42): (30, 30, 32),     # navy cards -> Adwaita dark
    (26, 32, 66): (36, 36, 40),
    (6, 7, 18): (0, 0, 0),
    (7, 8, 20): (0, 0, 0),
}


def swap_rgba(m):
    nums = [x.strip() for x in m.group(2).split(",")]
    key = tuple(int(float(n)) for n in nums[:3])
    if key not in rgb_map:
        return m.group(0)
    r, g, b = rgb_map[key]
    rest = ", ".join([str(r), str(g), str(b)] + nums[3:])
    return f"{m.group(1)}({rest})"


p = f"{site}/css/style.css"
s = open(p).read()
for old, new in hex_map.items():
    s = re.sub(re.escape(old) + r"\b", new, s, flags=re.I)
s = re.sub(r"(rgba?)\(([0-9., ]+)\)", swap_rgba, s)
# The top of the page: black with a soft green glow, not the galaxy picture.
s = s.replace("url(../images/desktop.webp) center top / cover no-repeat",
              "radial-gradient(ellipse 70% 60% at 50% 0%, rgba(38, 234, 117, 0.16), transparent 70%), #000")
open(p, "w").write(s)

open(f"{site}/css/man-style.css", "w").write("""/* MAN's look on top of style.css (written by dev-tools/website/man-style.py). */
:root {
  --man-green: #26ea75;
  --man-green-dark: #18b85c;
  --man-on-green: #04200f;
  --man-surface: #1e1e20;
  --man-surface-2: #242428;
  /* The template's GNOME blue (buttons are drawn with it) and links. */
  --gnomeblue: #26ea75;
  --blue2: #5cf09a;
  --blue3: #26ea75;
  --link: #5cf09a;
}

/* Green buttons get dark text (white on the logo green is hard to read). */
.btn.suggested, a.btn.suggested, .nav-donate, a.nav-donate {
  color: var(--man-on-green) !important;
  -webkit-text-fill-color: var(--man-on-green);
}

/* Calm, flat surfaces like MAN's own windows. */
.card, .feature, .feature-card, .step, .faq-item, .release-card {
  background-color: rgba(30, 30, 32, 0.78);
  border-color: rgba(255, 255, 255, 0.07);
}

::selection { background: rgba(38, 234, 117, 0.35); }
a:focus-visible, button:focus-visible { outline-color: var(--man-green); }
""")
print("restyled", p)
