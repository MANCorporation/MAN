#!/usr/bin/env python3
"""Give the MAN website the look of GNOME's own website (gnome.org): light
pages with GNOME blue, a dark version when the computer uses dark mode, the
gnome.org hero (blue gradient with its triangle pattern), flat cards, no
starfield. The green MAN logo stays as it is.

    gnome-style.py ORIGINAL_STYLE_CSS SITE_DIR

The site's style.css is the gnome.org template (kept as it is) plus a
"cosmic" dark-space layer written only for a dark background (white text,
white glass, violet glows). This rewrites that layer onto variables that
switch between light and dark, and writes SITE_DIR/css/style.css.
"""
import re
import sys

src, site = sys.argv[1], sys.argv[2]
css = open(src).read()
cut = css.index("/* ===========================================================\n   MAN THEME")
base, layer = css[:cut], css[cut:]

# Rules that stay as they are: GNOME's black top bar, text on blue buttons,
# the laptop picture.
KEEP = re.compile(r"\.tb-|\.topbar|cosmic-cta|cosmic-banner-btn|\.btn\.suggested|nav-donate|laptop|\.skip|pulse-dot")

accent = {
    "#7c5cff": "#3584e4", "#5835e5": "#1c71d8", "#a78bfa": "var(--g-accent)",
    "#c4b5fd": "var(--g-accent)", "#e9d5ff": "var(--g-accent)", "#93c5fd": "var(--g-accent)",
    "#38bdf8": "#3584e4", "#7dd3fc": "var(--g-accent)", "#c04bd8": "#1c71d8",
}
rgb = {
    (124, 92, 255): "53, 132, 228", (167, 139, 250): "53, 132, 228",
    (56, 189, 248): "53, 132, 228", (192, 75, 216): "28, 113, 216",
    (232, 121, 249): "53, 132, 228", (196, 181, 253): "53, 132, 228",
}
CARD = {(16, 20, 42): "var(--g-card)", (14, 18, 40): "var(--g-card)", (14, 17, 40): "var(--g-band)",
        (26, 32, 66): "var(--g-card-hover)", (28, 18, 64): "var(--g-band)"}
text = {
    "#ffffff": "var(--g-fg)", "#fff": "var(--g-fg)", "#f8fafc": "var(--g-fg)",
    "#f1f5f9": "var(--g-fg)", "#e2e8f0": "var(--g-fg)", "#cbd5e1": "var(--g-muted)",
    "#94a3b8": "var(--g-dim)", "#64748b": "var(--g-dim)",
}
bg = {"#070814": "var(--g-bg)", "#05060f": "var(--g-bg)", "#090a18": "var(--g-bg)",
      "#1a1622": "var(--g-card)"}


def fix_value(prop, val, keep):
    for k, v in accent.items():
        val = re.sub(re.escape(k) + r"\b", v, val, flags=re.I)
    def rgba(m):
        n = [x.strip() for x in m.group(1).split(",")]
        key = tuple(int(float(x)) for x in n[:3])
        a = n[3] if len(n) > 3 else "1"
        if key in rgb:
            return f"rgba({rgb[key]}, {a})"
        if key == (255, 255, 255) and not keep:
            return f"rgba(var(--g-ink), {a})"        # white glass -> ink glass
        if keep:
            return m.group(0)
        if key in CARD:
            return CARD[key]
        # Any other dark navy surface of the space theme: a GNOME card.
        if key != (0, 0, 0) and max(key) < 70 and key[2] > key[0]:
            return "var(--g-card)"
        if key in ((6, 7, 18), (7, 8, 20)):
            return f"rgba(var(--g-bg-rgb), {a})"
        return m.group(0)
    val = re.sub(r"rgba?\(([0-9., ]+)\)", rgba, val)
    if not keep:
        for k, v in bg.items():
            val = re.sub(re.escape(k) + r"\b", v, val, flags=re.I)
    if not keep and prop in ("color", "-webkit-text-fill-color", "fill", "stroke", "border-color", "caret-color"):
        for k, v in text.items():
            val = re.sub(re.escape(k) + r"\b", v, val, flags=re.I)
    if not keep and prop == "background" and re.fullmatch(r"\s*#fff(fff)?\s*", val, re.I):
        val = " var(--g-card)"
    return val


out, pos = [], 0
for m in re.finditer(r"([^{}]*)\{([^{}]*)\}", layer):
    sel, body = m.group(1), m.group(2)
    keep = bool(KEEP.search(sel))
    def decl(d):
        if ":" not in d:
            return d
        prop, val = d.split(":", 1)
        return prop + ":" + fix_value(prop.strip().lower(), val, keep)
    body = ";".join(decl(d) for d in body.split(";"))
    # Gradient text that started at white would vanish on a white page.
    if not keep and "background-clip: text" in body:
        body = re.sub(r"#fff(fff)?\b", "var(--g-fg)", body, flags=re.I)
    out.append(layer[pos:m.start()] + sel + "{" + body + "}")
    pos = m.end()
out.append(layer[pos:])
layer = "".join(out)

gnome = """
/* ===========================================================
   GNOME look (dev-tools/website/gnome-style.py): light like gnome.org,
   dark when the computer uses dark mode. The MAN logo stays green.
   =========================================================== */
:root {
  --g-bg: #ffffff;
  --g-bg-rgb: 255, 255, 255;
  --g-band: #f6f5f4;
  --g-fg: #241f31;
  --g-muted: #5e5c64;
  --g-dim: #77767b;
  --g-ink: 0, 0, 0;
  --g-card: #ffffff;
  --g-card-hover: #fafafa;
  --g-border: rgba(0, 0, 0, 0.09);
  --g-accent: #1c71d8;
  --g-shadow: 0 1px 2px rgba(0, 0, 0, 0.06), 0 4px 14px rgba(0, 0, 0, 0.07);
  --man-cyan: #1c71d8;
  --man: #3584e4;
  --man-dark: #1c71d8;
  --gnomeblue: #3584e4;
  --link: #1c71d8;
  --space-bg: var(--g-bg);
  --space-deep: var(--g-bg);
  --space-surface: var(--g-card);
  --space-surface-hover: var(--g-card-hover);
  --space-border: var(--g-border);
  --space-border-glow: rgba(53, 132, 228, 0.45);
  --text-main: var(--g-fg);
  --text-muted: var(--g-muted);
  --text-dim: var(--g-dim);
  --fg-color: var(--g-fg);
  --bg-color: var(--g-bg);
  --tile-bg: var(--g-card);
  --tile-mid: var(--g-card-hover);
  --btn-fg: var(--g-fg);
  --btn-bg: rgba(0, 0, 0, 0.06);
}
@media (prefers-color-scheme: dark) {
  :root {
    --g-bg: #222226;
    --g-bg-rgb: 34, 34, 38;
    --g-band: #1d1d20;
    --g-fg: #ffffff;
    --g-muted: #c0bfbc;
    --g-dim: #9a9996;
    --g-ink: 255, 255, 255;
    --g-card: #2e2e32;
    --g-card-hover: #36363a;
    --g-border: rgba(255, 255, 255, 0.08);
    --g-accent: #78aeed;
    --g-shadow: 0 1px 2px rgba(0, 0, 0, 0.3), 0 4px 14px rgba(0, 0, 0, 0.25);
    --link: #78aeed;
    --man-cyan: #78aeed;
    --btn-bg: rgba(255, 255, 255, 0.1);
  }
}

html { color-scheme: light dark; background-color: var(--g-bg); }
body { background: var(--g-bg); }
.cosmic-canvas { display: none; }

/* The gnome.org hero: soft blue gradient with GNOME's triangles. */
.hero {
  background: linear-gradient(to top, #ccdef9aa, #fff0), url(../images/tris-l.svg) bottom left no-repeat,
              linear-gradient(to top, #ccdef9, #ffffff) !important;
  color: var(--g-fg);
}
@media (prefers-color-scheme: dark) {
  .hero {
    background: linear-gradient(to top, #433583dd, #5494e000), url(../images/tris-d.svg) bottom left no-repeat,
                linear-gradient(to top, #433583, #5494e0) !important;
  }
}

/* Headings and text like gnome.org: solid, no glow. */
h1, h2, h3, h4 { color: var(--g-fg); }
h2 { background: none; -webkit-text-fill-color: currentColor; }
.cosmic-gradient-text {
  background: linear-gradient(135deg, #3584e4, #1c71d8);
  -webkit-background-clip: text; background-clip: text;
  -webkit-text-fill-color: transparent;
}
@media (prefers-color-scheme: dark) {
  .cosmic-gradient-text { background-image: linear-gradient(135deg, #99c1f1, #ffffff); }
}
a:hover { text-shadow: none; }
.hero h1, .hero p, .hero .punchline *, .title, .lead { text-shadow: none !important; }

/* "Try MAN": a calm grey band like gnome.org's sections. */
#download { background: var(--g-band) !important; border: 0 !important; box-shadow: none !important; }
.bubble-card { background: var(--g-band); border-color: var(--g-border); backdrop-filter: none; }
.bubble.strong { background: #3584e4; color: #ffffff; border-color: transparent; box-shadow: none; animation: none; }
.bubble.strong b, .bubble.strong small { color: #ffffff; -webkit-text-fill-color: #ffffff; }

/* The MAN logo keeps its green. */
#logo .logo-mark { color: #26ea75; filter: none; }
#logo a span { background: none; -webkit-text-fill-color: var(--g-fg); color: var(--g-fg); }

/* Flat GNOME cards. */
.tile, .feature, .bubble, .hero-banner, .faq details, .release-card, .dl-choice, .cosmic-tag, .mac-chip {
  box-shadow: var(--g-shadow);
  backdrop-filter: none;
}
.full { background: var(--g-band); }

/* GNOME's blue "suggested" buttons, white text. */
.btn.cosmic-cta, .hero .btn.suggested, .cosmic-banner-btn {
  background: #3584e4 !important;
  border-color: transparent !important;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15) !important;
  color: #ffffff !important;
}
.btn.cosmic-cta:hover, .hero .btn.suggested:hover, .cosmic-banner-btn:hover { background: #1c71d8 !important; }
.nav-donate { background: #3584e4 !important; color: #ffffff !important; box-shadow: none !important; }
"""
open(f"{site}/css/style.css", "w").write(base + layer + gnome)

# Colours written straight into the pages (style="color:#fff" for the dark
# theme) follow the light/dark colours too.
import glob
for page in glob.glob(f"{site}/*.html"):
    h = open(page).read()
    n = re.sub(r'(style="[^"]*?color:\s*)(#fff(?:fff)?|#cbd5e1)\b',
               lambda m: m.group(1) + ("var(--g-fg)" if m.group(2).lower().startswith("#fff") else "var(--g-muted)"), h, flags=re.I)
    if n != h:
        open(page, "w").write(n)
        print("page colours:", page.rsplit("/", 1)[1])
print("GNOME look written to", f"{site}/css/style.css")
