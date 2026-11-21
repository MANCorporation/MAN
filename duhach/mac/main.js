// DUHACH! for Mac: the native part, JavaScript for Automation (osascript -l JavaScript,
// no compiling). Two jobs:
//  • the menu bar item (started at login by /Library/LaunchAgents/com.mancorporation.
//    Duhach.plist, or with the app): Boot into MAN, Restore MAN Snapshot…, More
//    Settings…, Install MAN…, Quit
//  • the installer window (ui.html, DUHACH!'s pages in libadwaita's look): Terminal
//    runs MAN-Mac/install-man.sh with the chosen way to install.
// The disk logic is in duhach-core.js (tested on Linux with test-core.js).
// MAN reads and writes the same files on its EFI partition (EFI/MAN/): bootmenu.env
// (start-up options, read by GRUB), snapshots.json, restore-request, restore-result.
//   osascript -l JavaScript main.js <Resources folder> [--menubar]
ObjC.import('Cocoa');
ObjC.import('WebKit');

const sa = Application.currentApplication();
sa.includeStandardAdditions = true;
const LABEL = 'com.mancorporation.Duhach';

function readText(path) {
    const s = $.NSString.stringWithContentsOfFileEncodingError(path, $.NSUTF8StringEncoding, null);
    return s.isNil() ? '' : ObjC.unwrap(s);
}
function sh(cmd) {
    try { return sa.doShellScript(cmd); } catch (e) { return ''; }
}
function shx(cmd) { return sa.doShellScript(cmd); } // throws on failure
const q = (s) => "'" + String(s).replace(/'/g, "'\\''") + "'";
function b64(text) {
    return ObjC.unwrap($.NSString.alloc.initWithUTF8String(text).dataUsingEncoding($.NSUTF8StringEncoding)
        .base64EncodedStringWithOptions(0));
}
function errorText(e) { return String((e && e.message) || e).replace(/^Error: /, ''); }
function cancelled(e) { return /-128|User canceled|cancelled/i.test(String(e && (e.errorNumber || e.message || e))); }

let RES = '';
let MENUBAR = false;
const windows = []; // kept alive (JXA objects are released otherwise)

// ── facts ──
function lay() {
    try { return DuhachCore.layout(shx); } catch (e) { return null; }
}
function version() {
    return ObjC.unwrap($.NSBundle.mainBundle.objectForInfoDictionaryKey('CFBundleShortVersionString')) || '';
}
function facts() {
    const num = s => Math.floor(Number(s) || 0);
    const GB = 1000 * 1000 * 1000;
    const size = num(sh("diskutil info -plist / | plutil -extract APFSContainerSize raw -"));
    const free = num(sh("diskutil info -plist / | plutil -extract APFSContainerFree raw -"));
    return {
        demo: false,
        model: sh("system_profiler SPHardwareDataType | awk -F': ' '/Model Name/ {print $2; exit}'"),
        chip: sh("sysctl -n machdep.cpu.brand_string"),
        arm64: sh("uname -m") === 'arm64',
        macos: sh("sw_vers -productVersion") || '0',
        diskGB: Math.round(size / GB),
        freeGB: Math.floor(free / GB),
        charger: sh("pmset -g batt | head -n 1").indexOf('AC Power') >= 0,
        internet: sh("curl -s -o /dev/null -m 6 -w '%{http_code}' https://updates.cdn-apple.com/ || true") !== '000',
        // The stub volume of MAN's boot loader is called "MAN".
        loader: sh("diskutil apfs list | grep -cE 'Name: +MAN \\(' || true") !== '0',
        version: version(),
    };
}

// ── admin commands ──
function admin(cmd, prompt) {
    return sa.doShellScript(cmd, { administratorPrivileges: true, withPrompt: prompt });
}
// Runs `inner` in MAN's EFI/MAN folder (mounting the EFI partition if needed).
function espScript(L, inner) {
    const d = L.esp.dev;
    return 'set -e; d=' + d + '; mp=$(diskutil info -plist $d | plutil -extract MountPoint raw - 2>/dev/null || true); m=0; ' +
        'if [ -z "$mp" ]; then diskutil mount $d >/dev/null; m=1; mp=$(diskutil info -plist $d | plutil -extract MountPoint raw -); fi; ' +
        'mkdir -p "$mp/EFI/MAN"; cd "$mp/EFI/MAN"; ' + inner + '; cd /; if [ $m = 1 ]; then diskutil unmount $d >/dev/null || true; fi';
}
const SEP = '@@DUHACH@@';
function readEsp(L) {
    const out = admin(espScript(L, "cat snapshots.json 2>/dev/null || true; echo " + SEP +
        "; cat restore-result 2>/dev/null || true; echo " + SEP + "; cat bootmenu.env 2>/dev/null || true"),
        "DUHACH! needs your password to read MAN's settings.");
    const [snaps, result, env] = out.split(SEP).map(s => s.trim());
    let list = null;
    try { list = JSON.parse(snaps); } catch (e) { list = null; }
    let last = null;
    if (result) {
        const r = result.split('\n');
        last = { ok: r[0] === 'ok', when: r[1], detail: r[0] === 'ok' ? '' : r.slice(3).join(' ') };
    }
    return { list, last, env: DuhachCore.envParse(env) };
}
function writeEnv(L, env) {
    admin(espScript(L, "printf '%s' " + q(b64(DuhachCore.envBuild(env))) + " | base64 -D > .bootmenu.env.tmp && mv -f .bootmenu.env.tmp bootmenu.env"),
        "DUHACH! needs your password to change MAN's start-up options.");
}

// The Mac's password for bless (Apple Silicon asks the owner for start-up disk changes).
function askPassword(why) {
    const r = sa.displayDialog(why + '\n\nType the password you use to log in to this Mac.', {
        defaultAnswer: '', hiddenAnswer: true, buttons: ['Cancel', 'OK'], defaultButton: 'OK',
        withTitle: 'DUHACH!', withIcon: Path(RES + '/duhach.icns'),
    });
    return r.textReturned;
}
function bless(target, nextOnly, pw) {
    const user = shx('id -un');
    let mount = '/';
    let pre = '';
    if (target === 'man') {
        const L = lay();
        if (!L || !L.stub) throw new Error("MAN's boot loader was not found on this Mac.");
        const v = L.stub.volume;
        pre = 'mp=$(diskutil info -plist ' + v + ' | plutil -extract MountPoint raw - 2>/dev/null || true); ' +
            '[ -n "$mp" ] || { diskutil mount ' + v + ' >/dev/null; mp=$(diskutil info -plist ' + v + ' | plutil -extract MountPoint raw -); }; ';
        mount = '"$mp"';
    }
    const cmd = pre + "printf '%s' " + q(pw) + ' | /usr/sbin/bless --mount ' + mount + ' --setBoot' +
        (nextOnly ? ' --nextonly' : '') + ' --user ' + q(user) + ' --stdinpass';
    sa.doShellScript(cmd, { administratorPrivileges: true, userName: user, password: pw });
}
function openStartupDisk() {
    sh("open 'x-apple.systempreferences:com.apple.Startup-Disk-Settings.extension' || open /System/Library/PreferencePanes/StartupDisk.prefPane");
}
function restartMac() {
    // Like  > Restart: apps are asked to quit and can save.
    sh("osascript -e 'tell application \"loginwindow\" to «event aevtrrst»'");
}
function alert(title, text) {
    sa.activate();
    sa.displayAlert(title, { message: text || '', as: 'informational' });
}

// ── Boot into MAN ──
function bootIntoMan(skipQuestion) {
    try {
        if (!skipQuestion) {
            sa.activate();
            sa.displayDialog('The Mac restarts into MAN, this time only. Save your work first.', {
                withTitle: 'Boot into MAN?', buttons: ['Cancel', 'Restart'], defaultButton: 'Restart',
                withIcon: Path(RES + '/duhach.icns'),
            });
        }
        const pw = askPassword('DUHACH! needs your password to start MAN.');
        try {
            bless('man', true, pw);
        } catch (e) {
            if (cancelled(e)) return false;
            sa.displayDialog("macOS did not let DUHACH! choose MAN (" + errorText(e) + ").\n\nIn Startup Disk, choose MAN and click Restart.", {
                withTitle: 'DUHACH!', buttons: ['Open Startup Disk'], defaultButton: 'Open Startup Disk',
            });
            openStartupDisk();
            return false;
        }
        restartMac();
        return true;
    } catch (e) {
        if (!cancelled(e)) alert('MAN could not be started', errorText(e));
        return false;
    }
}

// ── the tools windows (Restore MAN Snapshot, More Settings) ──
function deleteRequestFile() { return sh('echo "$HOME"') + '/Library/Application Support/DUHACH/delete-man-request'; }
function deleteRequest() {
    const t = Number(sh('cat ' + q(deleteRequestFile()) + ' 2>/dev/null'));
    return t > 0 ? t : null;
}
function loginItem() {
    const out = sh('launchctl print-disabled gui/$(id -u) 2>/dev/null');
    return !new RegExp('"' + LABEL.replace(/\./g, '\\.') + '" => (disabled|true)').test(out);
}
function startupNow(L) {
    const cur = sh('/usr/sbin/bless --getBoot 2>/dev/null');
    if (!cur || !L || !L.stub) return 'macos';
    return cur.indexOf(L.stub.volume) >= 0 ? 'man' : 'macos';
}

function toolsData(view, esp) {
    const L = lay();
    const d = { view, version: version(), man: !!(L && L.esp), loginItem: loginItem() };
    if (!L || !L.esp) {
        if (view === 'snapshots') d.error = 'MAN is not on this Mac.';
        return d;
    }
    d.manSize = DuhachCore.gb(DuhachCore.manBytes(L));
    d.startup = startupNow(L);
    d.delete = DuhachCore.deleteState(deleteRequest(), Date.now());
    if (esp) {
        d.options = {};
        DuhachCore.OPTIONS.forEach(k => { d.options[k] = esp.env['man_' + k] === '1'; });
        if (view === 'snapshots') {
            if (!esp.list) {
                d.error = L.installed ? 'MAN has not shared its snapshots yet. Start MAN once, then try again.' : 'MAN is not installed on this Mac yet.';
            } else {
                d.snapshots = esp.list.snapshots || [];
                d.manVersion = esp.list.man_version ? 'MAN ' + esp.list.man_version : '';
            }
            d.lastResult = esp.last;
        }
    }
    return d;
}

function openTools(view) {
    let esp = null;
    const L = lay();
    if (L && L.esp) {
        try { esp = readEsp(L); } catch (e) { if (cancelled(e)) return; esp = null; }
    }
    const data = toolsData(view, esp);
    const title = view === 'snapshots' ? 'Restore MAN Snapshot' : 'More Settings';
    const w = openWebWindow('tools.html', data, title, 620, view === 'snapshots' ? 560 : 720, (m, web) => toolsAction(m, web, view));
    return w;
}

function refresh(web, view, note) {
    const L = lay();
    let esp = null;
    if (L && L.esp) { try { esp = readEsp(L); } catch (e) { esp = null; } }
    web.evaluateJavaScriptCompletionHandler('update(' + JSON.stringify(toolsData(view, esp)) + ')' +
        (note ? ';toast(' + JSON.stringify(note) + ')' : ''), null);
}

function toolsAction(m, web, view) {
    const L = lay();
    try {
        if (m.action === 'restore') {
            admin(espScript(L, "printf '%s\\n' " + q(m.id) + " > restore-request"),
                'DUHACH! needs your password to ask MAN to restore the snapshot.');
            if (!bootIntoMan(true)) {
                // Not restarted: take the request back, nothing changes.
                admin(espScript(L, 'rm -f restore-request'), 'DUHACH! needs your password.');
                refresh(web, view, 'Not restored: the Mac did not restart into MAN.');
            }
        } else if (m.action === 'option') {
            const env = readEsp(L).env;
            if (m.value) env['man_' + m.key] = '1'; else delete env['man_' + m.key];
            writeEnv(L, env);
            refresh(web, view, 'Saved. It takes effect the next time MAN starts.');
        } else if (m.action === 'startup') {
            const pw = askPassword('DUHACH! needs your password to change the start-up disk.');
            try {
                bless(m.value, false, pw);
                refresh(web, view, m.value === 'man' ? 'The Mac now starts MAN.' : 'The Mac now starts macOS.');
            } catch (e) {
                if (cancelled(e)) return refresh(web, view);
                openStartupDisk();
                refresh(web, view, 'Not changed by DUHACH!: choose it in Startup Disk.');
            }
        } else if (m.action === 'login') {
            sh('launchctl ' + (m.value ? 'enable' : 'disable') + ' gui/$(id -u)/' + LABEL);
            refresh(web, view, m.value ? 'DUHACH! starts when you log in.' : 'DUHACH! no longer starts when you log in.');
        } else if (m.action === 'copyinfo') {
            sa.setTheClipboardTo('DUHACH! ' + version() + ', macOS ' + sh('sw_vers -productVersion') + '\n\n' +
                sh('diskutil list') + '\n\n' + JSON.stringify(L, null, 2));
            refresh(web, view, 'Copied.');
        } else if (m.action === 'repair') {
            startInstaller(RES, '1', '');
        } else if (m.action === 'install') {
            openInstaller();
        } else if (m.action === 'delrequest') {
            const f = deleteRequestFile();
            sh('mkdir -p "$(dirname ' + q(f) + ')" && echo ' + Date.now() + ' > ' + q(f));
            refresh(web, view, 'Asked. MAN can be deleted in 24 hours.');
        } else if (m.action === 'delcancel') {
            sh('rm -f ' + q(deleteRequestFile()));
            refresh(web, view, 'Cancelled. MAN stays.');
        } else if (m.action === 'delete') {
            const st = DuhachCore.deleteState(deleteRequest(), Date.now());
            if (!m.immediate && st.state !== 'ready') return refresh(web, view, 'Not yet: the 24 hours are not over.');
            const cmds = DuhachCore.deleteCommands(L);
            // macOS starts by itself from now on; then Asahi's uninstall steps.
            const script = '/usr/sbin/bless --mount / --setBoot >/dev/null 2>&1 || true; ' +
                'diskutil unmount ' + L.esp.dev + ' >/dev/null 2>&1 || true; set -e; ' + cmds.join('; ');
            admin(script, 'DUHACH! needs your password to delete MAN.');
            sh('rm -f ' + q(deleteRequestFile()));
            refresh(web, view, 'MAN was deleted. macOS has its space back.');
        }
    } catch (e) {
        refresh(web, view, cancelled(e) ? 'Cancelled.' : 'Did not work: ' + errorText(e));
    }
}

// ── the installer window (unchanged pages) ──
function startInstaller(res, choice, size) {
    // A .command file opens in Terminal without asking for Automation permission.
    const base = sh('mktemp -t duhach');
    const file = base + '.command';
    const cmd = 'MAN_CHOICE=' + choice + ' MAN_SIZE=' + size + ' exec /bin/sh ' + q(res + '/MAN-Mac/install-man.sh');
    sh("printf '%s\\n' '#!/bin/sh' 'clear' " + q(cmd) + ' > ' + q(file) + ' && chmod 755 ' + q(file) +
        ' && rm -f ' + q(base) + ' && open -a Terminal ' + q(file));
}
function openInstaller() {
    return openWebWindow('ui.html', facts(), 'DUHACH!', 820, 680, (m) => {
        if (m.action === 'start') startInstaller(RES, String(m.choice), String(m.size || ''));
        else if (m.action === 'quit') closeFront();
    });
}
function closeFront() {
    const w = $.NSApp.keyWindow;
    if (!w.isNil()) w.performClose(null);
}

// ── web view windows ──
ObjC.registerSubclass({
    name: 'DuhachMessages',
    superclass: 'NSObject',
    protocols: ['WKScriptMessageHandler'],
    properties: { tag: 'id' },
    methods: {
        'userContentController:didReceiveScriptMessage:': {
            types: ['void', ['id', 'id']],
            implementation: function (controller, message) {
                // Later, outside WebKit's call: the page can show "Please wait" first.
                const json = $.NSString.alloc.initWithUTF8String(JSON.stringify(ObjC.deepUnwrap(message.body) || {}));
                this.performSelectorWithObjectAfterDelay('handle:', json, 0.05);
            },
        },
        'handle:': {
            types: ['void', ['id']],
            implementation: function (json) {
                const entry = windows.filter(w => w.handler.tag && ObjC.unwrap(w.handler.tag) === ObjC.unwrap(this.tag))[0];
                if (entry) entry.on(JSON.parse(ObjC.unwrap(json)), entry.web);
            },
        },
    },
});

let windowCount = 0;
function openWebWindow(file, data, title, width, height, on) {
    const style = $.NSWindowStyleMaskTitled | $.NSWindowStyleMaskClosable | $.NSWindowStyleMaskMiniaturizable |
        $.NSWindowStyleMaskResizable | $.NSWindowStyleMaskFullSizeContentView;
    const win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
        $.NSMakeRect(0, 0, width, height), style, $.NSBackingStoreBuffered, false);
    win.title = title;
    win.titlebarAppearsTransparent = true;
    win.titleVisibility = $.NSWindowTitleHidden;
    win.minSize = $.NSMakeSize(Math.min(width, 520), Math.min(height, 480));
    win.releasedWhenClosed = false;

    const config = $.WKWebViewConfiguration.alloc.init;
    const handler = $.DuhachMessages.alloc.init;
    handler.tag = $('w' + (++windowCount));
    config.userContentController.addScriptMessageHandlerName(handler, 'duhach');
    const web = $.WKWebView.alloc.initWithFrameConfiguration($.NSMakeRect(0, 0, width, height), config);
    web.autoresizingMask = $.NSViewWidthSizable | $.NSViewHeightSizable;
    win.contentView = web;

    const html = readText(RES + '/' + file);
    web.loadHTMLStringBaseURL(html.replace('/*DATA*/null', JSON.stringify(data)), $.NSURL.fileURLWithPath(RES));
    windows.push({ win, web, handler, on });
    win.center;
    win.makeKeyAndOrderFront(null);
    $.NSApp.activateIgnoringOtherApps(true);
    return win;
}

// ── the menu bar item ──
let statusItem = null;
const menuItems = {};
ObjC.registerSubclass({
    name: 'DuhachMenu',
    superclass: 'NSObject',
    protocols: ['NSMenuDelegate'],
    methods: {
        'menuWillOpen:': {
            types: ['void', ['id']],
            implementation: function (menu) {
                const L = lay();
                menuItems.boot.enabled = !!(L && L.stub);
                menuItems.restore.enabled = !!(L && L.installed);
                menuItems.install.title = L && L.installed ? 'Install or Repair MAN…' : 'Install MAN…';
            },
        },
        'bootMan:': { types: ['void', ['id']], implementation: function () { bootIntoMan(false); } },
        'restore:': { types: ['void', ['id']], implementation: function () { openTools('snapshots'); } },
        'settings:': { types: ['void', ['id']], implementation: function () { openTools('settings'); } },
        'install:': { types: ['void', ['id']], implementation: function () { openInstaller(); } },
    },
});
const menuTarget = $.DuhachMenu.alloc.init;

function addMenuBarItem() {
    statusItem = $.NSStatusBar.systemStatusBar.statusItemWithLength($.NSVariableStatusItemLength);
    const img = $.NSImage.alloc.initWithContentsOfFile(RES + '/menubar.png');
    if (!img.isNil()) {
        img.template = true;
        img.size = $.NSMakeSize(18, 18);
        statusItem.button.image = img;
    } else {
        statusItem.button.title = 'DUHACH!';
    }
    statusItem.button.toolTip = 'DUHACH!';
    const menu = $.NSMenu.alloc.initWithTitle('DUHACH!');
    menu.autoenablesItems = false;
    menu.delegate = menuTarget;
    const add = (key, title, action) => {
        const it = $.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(title, action, '');
        it.target = menuTarget;
        menu.addItem(it);
        if (key) menuItems[key] = it;
        return it;
    };
    add('boot', 'Boot into MAN', 'bootMan:');
    add('restore', 'Restore MAN Snapshot…', 'restore:');
    add('settings', 'More Settings…', 'settings:');
    menu.addItem($.NSMenuItem.separatorItem);
    add('install', 'Install MAN…', 'install:');
    menu.addItem($.NSMenuItem.separatorItem);
    menu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent('Quit DUHACH!', 'terminate:', ''));
    statusItem.menu = menu;
}

ObjC.registerSubclass({
    name: 'DuhachAppDelegate',
    superclass: 'NSObject',
    protocols: ['NSApplicationDelegate'],
    methods: {
        'applicationShouldTerminateAfterLastWindowClosed:': {
            types: ['bool', ['id']],
            implementation: function () { return !MENUBAR; },
        },
    },
});

function mainMenu() {
    // Quit (Cmd-Q) and Edit (so typing in fields can use Cmd-C/V).
    const main = $.NSMenu.alloc.init;
    const appItem = $.NSMenuItem.alloc.init;
    const appMenu = $.NSMenu.alloc.initWithTitle('DUHACH!');
    appMenu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent('Close Window', 'performClose:', 'w'));
    appMenu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent('Quit DUHACH!', 'terminate:', 'q'));
    appItem.submenu = appMenu;
    main.addItem(appItem);
    const editItem = $.NSMenuItem.alloc.init;
    const editMenu = $.NSMenu.alloc.initWithTitle('Edit');
    [['Cut', 'cut:', 'x'], ['Copy', 'copy:', 'c'], ['Paste', 'paste:', 'v'], ['Select All', 'selectAll:', 'a']]
        .forEach(([t, a, k]) => editMenu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(t, a, k)));
    editItem.submenu = editMenu;
    main.addItem(editItem);
    return main;
}

function run(argv) {
    RES = argv[0];
    eval(readText(RES + '/duhach-core.js').replace('var DuhachCore =', 'globalThis.DuhachCore ='));
    const wantMenubar = argv.indexOf('--menubar') >= 0;
    // Already in the menu bar (started at login)? Then this launch only shows the installer.
    // (The pattern does not match its own command line: [.] and [b].)
    const other = wantMenubar ? '' : sh("pgrep -f 'main[.]js.*--menu[b]ar' | head -n 1");
    MENUBAR = wantMenubar || !other;

    const app = $.NSApplication.sharedApplication;
    app.setActivationPolicy(MENUBAR ? $.NSApplicationActivationPolicyAccessory : $.NSApplicationActivationPolicyRegular);
    app.delegate = $.DuhachAppDelegate.alloc.init;
    app.mainMenu = mainMenu();
    if (MENUBAR) addMenuBarItem();
    // Opened by hand (not at login): show the installer window too.
    if (!wantMenubar) openInstaller();
    app.run;
}
