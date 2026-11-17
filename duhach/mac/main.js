// DUHACH! for Mac: the native part, JavaScript for Automation (osascript -l JavaScript,
// no compiling). Collects facts about this Mac, opens a macOS window with a web view
// showing ui.html (DUHACH!'s pages in libadwaita's look) and does what the page asks:
//   {action: "start", choice: "1|2|3", size: "<GB>"}  Terminal runs MAN-Mac/install-man.sh
//   {action: "quit"}
// Run by Contents/MacOS/DUHACH:  osascript -l JavaScript main.js <Resources folder>
ObjC.import('Cocoa');
ObjC.import('WebKit');

const sa = Application.currentApplication();
sa.includeStandardAdditions = true;

function sh(cmd) {
    try { return sa.doShellScript(cmd); } catch (e) { return ''; }
}
function q(s) { return "'" + String(s).replace(/'/g, "'\\''") + "'"; }

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
        version: ObjC.unwrap($.NSBundle.mainBundle.objectForInfoDictionaryKey('CFBundleShortVersionString')) || '',
    };
}

function startInstaller(res, choice, size) {
    // A .command file opens in Terminal without asking for Automation permission.
    const base = sh('mktemp -t duhach');
    const file = base + '.command';
    const run = 'MAN_CHOICE=' + choice + ' MAN_SIZE=' + size + ' exec /bin/sh ' + q(res + '/MAN-Mac/install-man.sh');
    sh("printf '%s\\n' '#!/bin/sh' 'clear' " + q(run) + ' > ' + q(file) + ' && chmod 755 ' + q(file) +
        ' && rm -f ' + q(base) + ' && open -a Terminal ' + q(file));
}

function run(argv) {
    const res = argv[0];
    const app = $.NSApplication.sharedApplication;
    app.setActivationPolicy($.NSApplicationActivationPolicyRegular);

    ObjC.registerSubclass({
        name: 'DuhachAppDelegate',
        protocols: ['NSApplicationDelegate'],
        methods: {
            'applicationShouldTerminateAfterLastWindowClosed:': {
                types: ['bool', ['id']],
                implementation: function (sender) { return true; },
            },
        },
    });
    ObjC.registerSubclass({
        name: 'DuhachMessages',
        protocols: ['WKScriptMessageHandler'],
        methods: {
            'userContentController:didReceiveScriptMessage:': {
                types: ['void', ['id', 'id']],
                implementation: function (controller, message) {
                    const m = ObjC.deepUnwrap(message.body) || {};
                    if (m.action === 'start') {
                        startInstaller(res, String(m.choice), String(m.size || ''));
                    } else if (m.action === 'quit') {
                        $.NSApp.terminate(null);
                    }
                },
            },
        },
    });
    const delegate = $.DuhachAppDelegate.alloc.init;
    app.delegate = delegate;

    // Menu: Quit (Cmd-Q) and Edit (so typing in the size field can use Cmd-C/V).
    const mainMenu = $.NSMenu.alloc.init;
    const appItem = $.NSMenuItem.alloc.init;
    const appMenu = $.NSMenu.alloc.initWithTitle('DUHACH!');
    appMenu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent('Quit DUHACH!', 'terminate:', 'q'));
    appItem.submenu = appMenu;
    mainMenu.addItem(appItem);
    const editItem = $.NSMenuItem.alloc.init;
    const editMenu = $.NSMenu.alloc.initWithTitle('Edit');
    [['Cut', 'cut:', 'x'], ['Copy', 'copy:', 'c'], ['Paste', 'paste:', 'v'], ['Select All', 'selectAll:', 'a']]
        .forEach(([t, a, k]) => editMenu.addItem($.NSMenuItem.alloc.initWithTitleActionKeyEquivalent(t, a, k)));
    editItem.submenu = editMenu;
    mainMenu.addItem(editItem);
    app.mainMenu = mainMenu;

    const style = $.NSWindowStyleMaskTitled | $.NSWindowStyleMaskClosable | $.NSWindowStyleMaskMiniaturizable |
        $.NSWindowStyleMaskResizable | $.NSWindowStyleMaskFullSizeContentView;
    const win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer(
        $.NSMakeRect(0, 0, 820, 680), style, $.NSBackingStoreBuffered, false);
    win.title = 'DUHACH!';
    win.titlebarAppearsTransparent = true;
    win.titleVisibility = $.NSWindowTitleHidden;
    win.minSize = $.NSMakeSize(520, 560);
    win.releasedWhenClosed = false;

    const config = $.WKWebViewConfiguration.alloc.init;
    const handler = $.DuhachMessages.alloc.init;
    config.userContentController.addScriptMessageHandlerName(handler, 'duhach');
    const web = $.WKWebView.alloc.initWithFrameConfiguration($.NSMakeRect(0, 0, 820, 680), config);
    web.autoresizingMask = $.NSViewWidthSizable | $.NSViewHeightSizable;
    win.contentView = web;

    const html = ObjC.unwrap($.NSString.stringWithContentsOfFileEncodingError(res + '/ui.html', $.NSUTF8StringEncoding, null));
    const page = html.replace('/*DATA*/null', JSON.stringify(facts()));
    web.loadHTMLStringBaseURL(page, $.NSURL.fileURLWithPath(res));

    win.center;
    win.makeKeyAndOrderFront(null);
    app.activateIgnoringOtherApps(true);
    app.run;
}
