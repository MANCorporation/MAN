// DUHACH! for Mac: the logic that does not need macOS itself (tested with Node on
// Linux, see test-core.js): reading diskutil's plists, finding MAN on the internal
// disk, the start-up options file and the commands for deleting MAN. main.js (JXA)
// loads this file with eval and gives it `run(cmd)` (a shell command's output).
//
// MAN on a Mac (Asahi layout, in disk order):
//   … macOS APFS container … | MAN's stub APFS container (volume "MAN": m1n1, the
//   boot loader macOS starts) | MAN's EFI partition (GRUB; EFI/MAN/… is shared
//   with MAN) | MAN's Linux partitions (/boot, the Btrfs root, "MAN extra") | … the
//   Mac's recovery container
var DuhachCore = (function () {
    'use strict';

    // ── plist (diskutil -plist) ──
    function unescapeXml(s) {
        return s.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"')
            .replace(/&apos;/g, "'").replace(/&#(\d+);/g, function (_m, n) { return String.fromCharCode(+n); })
            .replace(/&amp;/g, '&');
    }

    function parsePlist(xml) {
        var re = /<(\/?)(dict|array|key|string|integer|real|true|false|data|date)(\s*\/)?>([^<]*)/g;
        var tokens = [];
        var m;
        while ((m = re.exec(xml)) !== null) {
            tokens.push({ close: m[1] === '/', tag: m[2], empty: !!m[3], text: m[4] });
        }
        var i = 0;
        function value() {
            var t = tokens[i++];
            if (!t) throw new Error('plist: unexpected end');
            if (t.tag === 'dict') {
                var d = {};
                if (t.empty) return d;
                while (!(tokens[i].close && tokens[i].tag === 'dict')) {
                    var k = tokens[i++];
                    if (k.tag !== 'key' || k.close) throw new Error('plist: key expected');
                    if (!k.empty) i++; // </key>
                    d[unescapeXml(k.text)] = value();
                }
                i++;
                return d;
            }
            if (t.tag === 'array') {
                var a = [];
                if (t.empty) return a;
                while (!(tokens[i].close && tokens[i].tag === 'array')) a.push(value());
                i++;
                return a;
            }
            if (t.tag === 'true') return true;
            if (t.tag === 'false') return false;
            var text = t.empty ? '' : unescapeXml(t.text);
            if (!t.empty) i++; // its closing tag
            if (t.tag === 'integer') return parseInt(text, 10);
            if (t.tag === 'real') return parseFloat(text);
            return text;
        }
        return value();
    }

    // ── finding MAN ──
    var LINUX = /linux|0FC63DAF-8483-4772-8E79-3D69D8477DE4/i;

    function isEfi(p) { return /^EFI$|C12A7328-F81F-11D2-BA4B-00A0C93EC93B/i.test(p.content); }
    function isApfs(p) { return p.content === 'Apple_APFS'; }

    // run(cmd) -> stdout. Returns the internal disk's layout and where MAN is.
    function layout(run, disk) {
        disk = disk || 'disk0';
        var list = parsePlist(run('diskutil list -plist ' + disk));
        var whole = (list.AllDisksAndPartitions || [])[0] || {};
        var parts = (whole.Partitions || []).map(function (p) {
            var info = parsePlist(run('diskutil info -plist ' + p.DeviceIdentifier));
            return {
                dev: p.DeviceIdentifier,
                content: p.Content || '',
                size: p.Size || info.Size || 0,
                offset: info.PartitionMapPartitionOffset || 0,
                name: p.VolumeName || info.VolumeName || '',
            };
        }).sort(function (a, b) { return a.offset - b.offset; });

        var apfs = parsePlist(run('diskutil apfs list -plist'));
        var containers = (apfs.Containers || []).map(function (c) {
            return {
                ref: c.ContainerReference,
                stores: (c.PhysicalStores || []).map(function (s) { return s.DeviceIdentifier; }),
                volumes: (c.Volumes || []).map(function (v) {
                    return { dev: v.DeviceIdentifier, name: v.Name, roles: v.Roles || [] };
                }),
            };
        });
        function containerOf(dev) {
            for (var i = 0; i < containers.length; i++) {
                if (containers[i].stores.indexOf(dev) >= 0) return containers[i];
            }
            return null;
        }

        var root = parsePlist(run('diskutil info -plist /'));
        var rootStores = (root.APFSPhysicalStores || []).map(function (s) { return s.APFSPhysicalStore; });

        var res = { disk: disk, parts: parts, stub: null, esp: null, linux: [], macos: null, installed: false };
        parts.forEach(function (p, i) {
            if (rootStores.indexOf(p.dev) >= 0) res.macos = { store: p.dev, size: p.size, container: (containerOf(p.dev) || {}).ref };
            if (!isApfs(p) || res.stub) return;
            var c = containerOf(p.dev);
            var vol = c && c.volumes.filter(function (v) { return v.name === 'MAN'; })[0];
            if (!vol) return;
            res.stub = { store: p.dev, container: c.ref, volume: vol.dev, size: p.size, index: i };
        });
        if (res.stub) {
            var next = parts[res.stub.index + 1];
            if (next && isEfi(next)) {
                res.esp = { dev: next.dev, size: next.size, name: next.name };
                for (var j = res.stub.index + 2; j < parts.length && LINUX.test(parts[j].content); j++) {
                    res.linux.push({ dev: parts[j].dev, size: parts[j].size, name: parts[j].name });
                }
            }
            res.installed = res.linux.length > 0;
        }
        return res;
    }

    // MAN's space in bytes (what deleting gives back to macOS).
    function manBytes(lay) {
        var n = 0;
        if (lay.stub) n += lay.stub.size;
        if (lay.esp) n += lay.esp.size;
        lay.linux.forEach(function (p) { n += p.size; });
        return n;
    }

    // ── the start-up options: GRUB's environment block (exactly 1024 bytes) ──
    var ENV_HEAD = '# GRUB Environment Block\n';

    function envParse(text) {
        var o = {};
        (text || '').split('\n').forEach(function (line) {
            if (!line || line[0] === '#') return;
            var k = line.indexOf('=');
            if (k > 0) o[line.slice(0, k)] = line.slice(k + 1);
        });
        return o;
    }

    function envBuild(o) {
        var s = ENV_HEAD;
        Object.keys(o).sort().forEach(function (k) {
            if (o[k] !== undefined && o[k] !== null && o[k] !== '') s += k + '=' + o[k] + '\n';
        });
        if (s.length > 1024) throw new Error('start-up options too long');
        while (s.length < 1024) s += '#';
        return s;
    }

    var OPTIONS = ['verbose', 'debug', 'always_menu'];

    // ── deleting MAN (Asahi's documented uninstall, then macOS takes the space) ──
    function deleteCommands(lay) {
        if (!lay.stub || !lay.esp) throw new Error('MAN was not found on this Mac');
        if (!lay.macos) throw new Error("macOS's own container was not found");
        var cmds = [];
        lay.linux.slice().reverse().forEach(function (p) { cmds.push('diskutil eraseVolume free free ' + p.dev); });
        cmds.push('diskutil eraseVolume free free ' + lay.esp.dev);
        cmds.push('diskutil apfs deleteContainer ' + lay.stub.store);
        cmds.push('diskutil apfs resizeContainer ' + lay.macos.store + ' 0');
        return cmds;
    }

    // ── the 24 hours ──
    var WAIT_MS = 24 * 3600 * 1000;
    function deleteState(requestedAt, now) {
        if (!requestedAt) return { state: 'none' };
        var ready = requestedAt + WAIT_MS;
        return { state: now >= ready ? 'ready' : 'waiting', readyAt: ready };
    }

    function quote(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }

    function gb(n) { return Math.round(n / 1e9) + ' GB'; }

    return {
        parsePlist: parsePlist, layout: layout, manBytes: manBytes,
        envParse: envParse, envBuild: envBuild, OPTIONS: OPTIONS,
        deleteCommands: deleteCommands, deleteState: deleteState, WAIT_MS: WAIT_MS,
        quote: quote, gb: gb,
    };
})();
if (typeof module !== 'undefined') module.exports = DuhachCore;
