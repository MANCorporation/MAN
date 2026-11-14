#!/usr/bin/env python3
"""Patch the Asahi installer's urlcache.py (MIT) so it can download MAN's package from
GitHub releases. GitHub answers with a redirect to a signed link that expires after a
few minutes; the original code dropped the link's query string (the signature), never
re-resolved the redirect, and did not check the HTTP status (an error page would have
been written into the partition as data).
  patch-urlcache.py path/to/urlcache.py"""
import sys
p = sys.argv[1]
s = open(p).read()

def rep(old, new):
    global s
    if old not in s:
        sys.exit(f"patch-urlcache: pattern not found:\n{old}")
    s = s.replace(old, new, 1)

rep('''    def __init__(self, url):
        self.url_str = url
        self.url = parse.urlparse(url)
        self.con = None
        self.size = self.get_size()''', '''    # MAN: re-resolve redirects (signed GitHub links) after this many seconds.
    REDIRECT_TTL = 120

    def __init__(self, url):
        self.url_str = url
        self.orig_url = parse.urlparse(url)
        self.url = self.orig_url
        self.resolved_at = 0
        self.con = None
        self.size = self.get_size()''')

rep('''    def get_size(self):
        for i in range(10):
            con = self.get_con()
            con.request("HEAD", self.url.path, headers={"Connection":" keep-alive"})
            res = con.getresponse()
            res.read()
            loc = res.getheader("Location", None)
            if loc is not None:
                self.url = parse.urlparse(loc)
                self.con = None
                continue
            return int(res.getheader("Content-length"))

        raise Exception("Maximum number of redirects reached")''', '''    def target(self, extra=None):
        # MAN: keep the query string (signed links need it).
        q = "&".join(x for x in (self.url.query, extra) if x)
        return self.url.path + (f"?{q}" if q else "")

    def get_size(self):
        self.url = self.orig_url
        self.con = None
        for i in range(10):
            con = self.get_con()
            con.request("HEAD", self.target(), headers={"Connection":" keep-alive"})
            res = con.getresponse()
            res.read()
            loc = res.getheader("Location", None)
            if loc is not None and 300 <= res.status < 400:
                self.url = parse.urlparse(parse.urljoin(self.url.geturl(), loc))
                self.con = None
                continue
            if res.status != 200:
                raise Exception(f"HTTP {res.status} for {self.url_str}")
            self.resolved_at = time.time()
            return int(res.getheader("Content-length"))

        raise Exception("Maximum number of redirects reached")''')

rep('''    def get_partial(self, off, size, bypass_cache=False):
        path = self.url.path
        if bypass_cache:
            path += f"?{random.random()}"

        res = None
        try:
            con = self.get_con()''', '''    def get_partial(self, off, size, bypass_cache=False):
        if self.url != self.orig_url and time.time() - self.resolved_at > self.REDIRECT_TTL:
            self.get_size()
        path = self.target(f"{random.random()}" if bypass_cache and self.url == self.orig_url else None)

        res = None
        try:
            con = self.get_con()''')

rep('''            res = con.getresponse()
            d = res.read()
        except Exception as e:''', '''            res = con.getresponse()
            d = res.read()
            if res.status != 206 and not (res.status == 200 and off == 0 and len(d) == size):
                # Expired signed link or another error: resolve again on the retry.
                self.resolved_at = 0
                self.con = None
                raise Exception(f"HTTP {res.status}")
        except Exception as e:''')

open(p, "w").write(s)
print("patched", p)
