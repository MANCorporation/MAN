#!/usr/bin/env python3
"""Builds the MAN website (GNOME / Adwaita look) from dev-tools/website/man-site.

    build-site.py OUT_DIR

Pages are pages/<name>.html: the first line is
    <!-- title: ... | desc: ... | nav: overview -->
then the page's content. The shared header bar, view switcher and footer are
added here. Shorthand in pages: {i:go-home} is an Adwaita symbolic icon,
{tile:blue:starred} an icon on a coloured tile like GNOME's settings rows.
Assets (css, js, icons, fonts, images, data) are copied as they are.
"""
import os
import re
import shutil
import sys

here = os.path.dirname(os.path.abspath(__file__))
src = os.path.join(here, "man-site")
out = sys.argv[1]
os.makedirs(out, exist_ok=True)

NAV = [
    ("overview", "index.html", "Overview", "go-home"),
    ("download", "download.html", "Get MAN", "folder-download"),
    ("whatsnew", "releases.html", "What's New", "software-update-available"),
    ("apps", "apps.html", "Apps", "view-app-grid"),
    ("help", "help.html", "Questions", "help-about"),
]


def icon(name, cls=""):
    u = f"url(icons/{name}-symbolic.svg)"
    return f'<span class="icon{(" " + cls) if cls else ""}" style="-webkit-mask-image:{u};mask-image:{u}" aria-hidden="true"></span>'


def expand(text):
    text = re.sub(r"\{tile:([a-z]+):([a-z0-9-]+)\}",
                  lambda m: f'<span class="tile" style="--c:var(--tile-{m.group(1)})">{icon(m.group(2))}</span>', text)
    text = re.sub(r"\{stile:([a-z]+):([a-z0-9-]+)\}",
                  lambda m: f'<span class="tile small" style="--c:var(--tile-{m.group(1)})">{icon(m.group(2))}</span>', text)
    return re.sub(r"\{i:([a-z0-9-]+)\}", lambda m: icon(m.group(1)), text)


def page(name, title, desc, nav, body):
    links = "".join(
        f'<a href="{href}"{" aria-current=\"page\"" if key == nav else ""}>{icon(ic)}<span>{label}</span></a>'
        for key, href, label, ic in NAV)
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <meta name="description" content="{desc}">
  <meta property="og:title" content="{title}">
  <meta property="og:description" content="{desc}">
  <meta property="og:image" content="images/card.png">
  <link rel="icon" type="image/png" href="images/favicon.png">
  <link rel="apple-touch-icon" href="images/apple-touch-icon.png">
  <link rel="stylesheet" href="css/adwaita.css?v=1">
  <script src="js/site.js?v=1" defer></script>
</head>
<body>
<a class="skip" href="#main">Skip to content</a>
<header class="headerbar">
  <a class="brand" href="index.html" aria-label="MAN home"><img src="images/man-logo.svg" alt=""><span>MAN</span></a>
  <nav class="viewswitcher" aria-label="Pages">{links}</nav>
  <div class="hb-end">
    <button class="btn flat" id="textsize" type="button" aria-pressed="false" title="Make the text bigger">A+</button>
    <a class="btn{" suggested" if nav == "donate" else ""}" href="donate.html">{icon("emote-love")}<span>Donate</span></a>
  </div>
</header>
<main id="main">
{expand(body)}
</main>
<footer>
  <div class="clamp">
    <nav aria-label="More">
      <a href="index.html">Overview</a><a href="download.html">Get MAN</a><a href="releases.html">What's New</a>
      <a href="apps.html">Apps</a><a href="help.html">Questions</a><a href="donate.html">Donate</a>
      <a href="https://github.com/MANCorporation/MAN">MAN on GitHub</a>
    </nav>
    <p>© 2026 MAN Corporation. MAN is built on GNOME and other open-source software, each under its own license.</p>
    <p>Icons from GNOME's Adwaita icon theme (CC BY-SA 3.0). Text in Adwaita Sans (SIL Open Font License 1.1).
    Mac support uses the work of the Asahi Linux project. GNOME is a trademark of the GNOME Foundation;
    Windows of Microsoft; Mac, macOS and Apple of Apple Inc. MAN is not made by or affiliated with them.</p>
  </div>
</footer>
<nav class="switcherbar" aria-label="Pages">{links}</nav>
</body>
</html>
"""


for d in ("css", "js", "icons", "fonts", "images", "data"):
    s = os.path.join(src, d)
    if os.path.isdir(s):
        shutil.copytree(s, os.path.join(out, d), dirs_exist_ok=True)
for f in sorted(os.listdir(os.path.join(src, "pages"))):
    text = open(os.path.join(src, "pages", f)).read()
    head, body = text.split("\n", 1)
    meta = dict(kv.strip().split(":", 1) for kv in head.strip()[4:-3].split("|"))
    meta = {k.strip(): v.strip() for k, v in meta.items()}
    open(os.path.join(out, f), "w").write(page(f, meta["title"], meta["desc"], meta["nav"], body))
    print("built", f)
