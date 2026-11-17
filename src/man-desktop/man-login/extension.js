// MAN Login: the login and lock screens in MAN's look.
//
// Login screen (GDM): the user's desktop background, users side by side as
// large round pictures, the password field under the chosen user, and
// Suspend / Restart / Power Off at the bottom.
// Lock screen: the user's background without blur, the same large picture.
// Desktop session: keeps a copy of the user's login screen background where
// the login screen can read it (/var/lib/man-login, written by
// /usr/libexec/man/man-login-background through pkexec).
import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Layout from 'resource:///org/gnome/shell/ui/layout.js';
import {UnlockDialog} from 'resource:///org/gnome/shell/ui/unlockDialog.js';
import {ScreenShield} from 'resource:///org/gnome/shell/ui/screenShield.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const STORE = '/var/lib/man-login';
const HELPER = '/usr/libexec/man/man-login-background';
const KINDS = ['jpg', 'png', 'webp'];
const FADE_MS = 400;

// GNOME Shell's own translations, so the buttons speak every language MAN has.
const shellText = s => GLib.dgettext('gnome-shell', s);

function fileExists(path) {
    return path && GLib.file_test(path, GLib.FileTest.EXISTS);
}

/** The stored login background of a user, or null. */
function storedBackground(userName) {
    if (!userName || !/^[a-z_][a-z0-9_.-]*\$?$/i.test(userName))
        return null;
    for (const k of KINDS) {
        const p = `${STORE}/${userName}.${k}`;
        if (fileExists(p))
            return p;
    }
    return null;
}

function lastUser() {
    try {
        const [ok, bytes] = GLib.file_get_contents(`${STORE}/last`);
        return ok ? new TextDecoder().decode(bytes).trim() : null;
    } catch {
        return null;
    }
}

/** Path of the picture an org.gnome.desktop.background URI points at. */
function picturePath(uri) {
    if (!uri)
        return null;
    let path;
    try {
        path = Gio.File.new_for_uri(uri).get_path();
    } catch {
        return null;
    }
    if (path && path.endsWith('.xml')) {
        // A changing background: its first picture.
        try {
            const [ok, bytes] = GLib.file_get_contents(path);
            const m = ok && new TextDecoder().decode(bytes).match(/<file>\s*([^<]+?)\s*<\/file>/);
            path = m ? m[1] : null;
        } catch {
            path = null;
        }
    }
    return fileExists(path) ? path : null;
}

const MAN_WALLPAPER = '/usr/share/backgrounds/man/wallpaper.jpg';

/** The login screen's own background setting, else MAN's wallpaper. */
function systemDefaultBackground() {
    const s = new Gio.Settings({schema_id: 'org.gnome.desktop.background'});
    const dark = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'}).get_string('color-scheme') === 'prefer-dark';
    const candidates = [
        dark ? s.get_string('picture-uri-dark') : null,
        s.get_string('picture-uri'),
        s.get_default_value('picture-uri')?.unpack(),
    ];
    for (const uri of candidates) {
        const path = picturePath(uri);
        if (path)
            return path;
    }
    return fileExists(MAN_WALLPAPER) ? MAN_WALLPAPER : null;
}

/** Settings > Appearance > Blur > Lock Screen is on (MAN Blur then blurs it). */
function lockBlurWanted() {
    const source = Gio.SettingsSchemaSource.get_default();
    if (!source.lookup('com.man.desktop.blur.lockscreen', true))
        return false;
    const shell = new Gio.Settings({schema_id: 'org.gnome.shell'});
    return shell.get_strv('enabled-extensions').includes('man-blur@man.invalid') &&
        new Gio.Settings({schema_id: 'com.man.desktop.blur.lockscreen'}).get_boolean('blur');
}

/** Full-screen picture layers on every monitor, cross-fading on change. */
class Backdrop {
    constructor(parent) {
        this._parent = parent;
        this._path = null;
        this._layers = [];
        this._build();
        Main.layoutManager.connectObject('monitors-changed', () => this._build(), this);
    }

    _build() {
        this._layers.forEach(l => l.destroy());
        this._layers = Main.layoutManager.monitors.map((m, i) => {
            const layer = new St.Widget({style_class: 'man-login-backdrop', reactive: false});
            layer.add_constraint(new Layout.MonitorConstraint({index: i}));
            this._parent.insert_child_at_index(layer, 0);
            return layer;
        });
        const path = this._path;
        this._path = null;
        this.show(path, false);
    }

    show(path, animate = true) {
        if (path === this._path)
            return;
        this._path = path;
        for (const layer of this._layers) {
            const style = path ? `background-image: url("file://${path}");` : '';
            if (!animate) {
                layer.style = style;
                continue;
            }
            // Old picture fades out on a copy, new one fades in.
            const old = new St.Widget({style_class: 'man-login-backdrop', style: layer.style, reactive: false});
            old.add_constraint(new Layout.MonitorConstraint({index: this._layers.indexOf(layer)}));
            this._parent.insert_child_above(old, layer);
            layer.style = style;
            old.ease({opacity: 0, duration: FADE_MS, mode: Clutter.AnimationMode.EASE_OUT_QUAD, onComplete: () => old.destroy()});
        }
    }

    destroy() {
        Main.layoutManager.disconnectObject(this);
        this._layers.forEach(l => l.destroy());
        this._layers = [];
    }
}

/** Suspend, Restart and Power Off under the user list (login screen only). */
class PowerButtons {
    constructor(parent) {
        this._area = new St.Widget({layout_manager: new Clutter.BinLayout(), reactive: false});
        this._area.add_constraint(new Layout.MonitorConstraint({primary: true}));
        this._box = new St.BoxLayout({
            style_class: 'man-login-power',
            x_align: Clutter.ActorAlign.CENTER,
            y_align: Clutter.ActorAlign.END,
            x_expand: true,
            y_expand: true,
        });
        this._area.add_child(this._box);
        for (const [icon, label, method] of [
            ['weather-clear-night-symbolic', shellText('Suspend'), 'Suspend'],
            ['view-refresh-symbolic', shellText('Restart'), 'Reboot'],
            ['system-shutdown-symbolic', shellText('Power Off'), 'PowerOff'],
        ])
            this._box.add_child(this._button(icon, label, method));
        parent.add_child(this._area);
    }

    _button(iconName, text, method) {
        const box = new St.BoxLayout({orientation: Clutter.Orientation.VERTICAL, style_class: 'man-login-power-item'});
        const circle = new St.Bin({style_class: 'man-login-power-circle', x_align: Clutter.ActorAlign.CENTER});
        circle.child = new St.Icon({icon_name: iconName, style_class: 'man-login-power-icon'});
        box.add_child(circle);
        box.add_child(new St.Label({text, style_class: 'man-login-power-label', x_align: Clutter.ActorAlign.CENTER}));
        const button = new St.Button({
            style_class: 'man-login-power-button',
            child: box,
            can_focus: true,
            reactive: true,
            accessible_name: text,
        });
        button.connect('clicked', () => {
            // logind asks for a password itself when other people are logged in.
            Gio.DBus.system.call('org.freedesktop.login1', '/org/freedesktop/login1',
                'org.freedesktop.login1.Manager', method, new GLib.Variant('(b)', [true]),
                null, Gio.DBusCallFlags.ALLOW_INTERACTIVE_AUTHORIZATION, -1, null,
                (conn, res) => {
                    try {
                        conn.call_finish(res);
                    } catch (e) {
                        logError(e, `MAN Login: ${method}`);
                    }
                });
        });
        return button;
    }

    setVisible(visible) {
        this._box.ease({opacity: visible ? 255 : 0, duration: 250});
        this._box.reactive = visible;
    }

    destroy() {
        this._area.destroy();
    }
}

/** The GDM login dialog in MAN's layout. */
class LoginLook {
    constructor(dialog, group) {
        this._dialog = dialog;
        this._group = group;
        group.add_style_class_name('man-login');
        this._backdrop = new Backdrop(group);
        this._power = new PowerButtons(group);
        this._showFor(null, false);

        // Users side by side.
        const list = dialog._userList;
        list._box.orientation = Clutter.Orientation.HORIZONTAL;
        list.vscrollbar_policy = St.PolicyType.NEVER;
        list.overlay_scrollbars = true;
        const fit = () => {
            // A scroll bar only when the users do not fit side by side.
            list.hscrollbar_policy = list._items.size > 5 ? St.PolicyType.AUTOMATIC : St.PolicyType.NEVER;
        };
        for (const item of list._items.values())
            this._styleItem(item);
        fit();
        list.connectObject('item-added', (l, item) => {
            this._styleItem(item);
            fit();
        }, this);

        dialog._notListedButton.x_align = Clutter.ActorAlign.CENTER;

        // No logo: the buttons are at the bottom.
        const logo = dialog._logoBin;
        logo.hide();
        logo.connectObject('notify::visible', () => logo.visible && logo.hide(), this);

        // Background of the chosen user; back to the default with the list.
        const prompt = dialog._authPrompt;
        this._origSetUser = prompt.setUser;
        const self = this;
        prompt.setUser = function (user) {
            self._origSetUser.call(this, user);
            self._showFor(user?.get_user_name() ?? null);
        };
        dialog._userSelectionBox.connectObject('notify::visible', () => {
            if (dialog._userSelectionBox.visible)
                this._showFor(null);
        }, this);
    }

    _styleItem(item) {
        item.add_style_class_name('man-login-user');
        item.x_expand = false;
        const widget = item._userWidget;
        widget.orientation = Clutter.Orientation.VERTICAL;
        widget.x_align = Clutter.ActorAlign.CENTER;
        widget.remove_style_class_name('horizontal');
        widget.add_style_class_name('vertical');
        if (widget._label) {
            widget._label.x_align = Clutter.ActorAlign.CENTER;
            widget._label.x_expand = true;
        }
    }

    /** The chosen user's background; with no user, the last one who logged in. */
    _showFor(userName, animate = true) {
        const path = storedBackground(userName) ?? storedBackground(lastUser()) ?? systemDefaultBackground();
        this._backdrop.show(path, animate);
    }

    destroy() {
        this._dialog._authPrompt.setUser = this._origSetUser;
        this._dialog._userList.disconnectObject(this);
        this._dialog._userSelectionBox.disconnectObject(this);
        this._dialog._logoBin.disconnectObject(this);
        this._dialog._userList._box.orientation = Clutter.Orientation.VERTICAL;
        this._group.remove_style_class_name('man-login');
        this._backdrop.destroy();
        this._power.destroy();
    }
}

/** Desktop session: copy the login background for the login screen. */
class BackgroundSync {
    constructor(settings) {
        this._settings = settings;
        this._bg = new Gio.Settings({schema_id: 'org.gnome.desktop.background'});
        this._iface = new Gio.Settings({schema_id: 'org.gnome.desktop.interface'});
        this._stamp = GLib.build_filenamev([GLib.get_user_cache_dir(), 'man-login', 'synced']);
        const later = () => this._schedule();
        this._settings.connectObject('changed', later, this);
        this._bg.connectObject('changed::picture-uri', later, 'changed::picture-uri-dark', later, this);
        this._iface.connectObject('changed::color-scheme', later, this);
        this._schedule(true);
    }

    _picture() {
        if (!this._settings.get_boolean('use-desktop-background'))
            return picturePath(this._settings.get_string('picture-uri'));
        const dark = this._iface.get_string('color-scheme') === 'prefer-dark';
        return (dark && picturePath(this._bg.get_string('picture-uri-dark'))) || picturePath(this._bg.get_string('picture-uri'));
    }

    _schedule(login = false) {
        if (this._timeout)
            GLib.source_remove(this._timeout);
        this._timeout = GLib.timeout_add_seconds(GLib.PRIORITY_LOW, login ? 5 : 2, () => {
            this._timeout = 0;
            this._sync(login).catch(e => logError(e, 'MAN Login: background'));
            return GLib.SOURCE_REMOVE;
        });
    }

    async _sync(login) {
        const path = this._picture();
        let key = '';
        let bytes = null;
        if (path) {
            const info = Gio.File.new_for_path(path).query_info('standard::size,time::modified', 0, null);
            key = `${path}\n${info.get_size()}\n${info.get_modification_date_time()?.to_unix() ?? 0}`;
        }
        let old = '';
        try {
            old = new TextDecoder().decode(GLib.file_get_contents(this._stamp)[1]);
        } catch {}
        if (key === old) {
            // Same picture: only tell the login screen who logged in last.
            if (login)
                await this._run(['last'], null);
            return;
        }
        if (path)
            bytes = GLib.file_get_contents(path)[1];
        if (await this._run(path ? ['set'] : ['clear'], bytes)) {
            GLib.mkdir_with_parents(GLib.path_get_dirname(this._stamp), 0o700);
            GLib.file_set_contents(this._stamp, key);
        }
    }

    _run(args, bytes) {
        return new Promise(resolve => {
            let proc;
            try {
                // With a stdin pipe, communicate() needs data: GLib aborts the
                // whole Shell on an empty one. No data, no pipe.
                const stdin = bytes ? Gio.SubprocessFlags.STDIN_PIPE : Gio.SubprocessFlags.NONE;
                proc = Gio.Subprocess.new(['pkexec', HELPER, ...args],
                    stdin | Gio.SubprocessFlags.STDOUT_SILENCE | Gio.SubprocessFlags.STDERR_PIPE);
            } catch (e) {
                logError(e, 'MAN Login: helper');
                resolve(false);
                return;
            }
            proc.communicate_async(bytes ? new GLib.Bytes(bytes) : null, null, (p, res) => {
                try {
                    const [, , err] = p.communicate_finish(res);
                    const ok = p.get_successful();
                    if (!ok)
                        log(`MAN Login: helper failed: ${err ? new TextDecoder().decode(err.toArray()) : ''}`);
                    resolve(ok);
                } catch (e) {
                    logError(e, 'MAN Login: helper');
                    resolve(false);
                }
            });
        });
    }

    destroy() {
        if (this._timeout)
            GLib.source_remove(this._timeout);
        this._settings.disconnectObject(this);
        this._bg.disconnectObject(this);
        this._iface.disconnectObject(this);
    }
}

export default class ManLoginExtension extends Extension {
    enable() {
        const mode = Main.sessionMode.currentMode;
        this._isGreeter = Main.sessionMode.isGreeter;

        // The login dialog may not exist yet.
        const self = this;
        this._origEnsure = ScreenShield.prototype._ensureUnlockDialog;
        ScreenShield.prototype._ensureUnlockDialog = function (...args) {
            const ret = self._origEnsure.apply(this, args);
            self._decorate();
            return ret;
        };
        this._decorate();

        if (mode === 'user' || Main.sessionMode.parentMode === 'user') {
            if (!this._sync)
                this._sync = new BackgroundSync(this.getSettings('com.man.desktop.login'));
        }
    }

    _decorate() {
        const shield = Main.screenShield;
        const dialog = shield?._dialog;
        if (!dialog)
            return;
        if (this._isGreeter) {
            if (!this._look && shield._lockDialogGroup)
                this._look = new LoginLook(dialog, shield._lockDialogGroup);
        } else if (dialog instanceof UnlockDialog && !dialog._manLogin && !lockBlurWanted()) {
            // Lock screen: the background as it is, no blur. Set on this
            // dialog itself, so no other change to the lock screen undoes it.
            dialog._manLogin = true;
            dialog._updateBackgroundEffects = function () {
                for (const widget of this._backgroundGroup) {
                    const effect = widget.get_effect('blur');
                    if (effect)
                        effect.set({brightness: 0.9, radius: 0});
                }
            };
            dialog._updateBackgroundEffects();
        }
    }

    disable() {
        // Called when the lock screen opens only if this extension did not
        // support it; it does, so this means the extension is really off.
        ScreenShield.prototype._ensureUnlockDialog = this._origEnsure;
        this._look?.destroy();
        this._look = null;
        this._sync?.destroy();
        this._sync = null;
    }
}
