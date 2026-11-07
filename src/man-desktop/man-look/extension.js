// MAN's look for system dialogs: stylesheet.css, and the dialog's action button
// (the last one, as in macOS alerts) marked so that it can be blue. Also: Resources in
// the System folder, and the live session starting on the desktop.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import * as Dialog from 'resource:///org/gnome/shell/ui/dialog.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension, InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';

// Once per Shell start: enabling again (after a screen lock) must not close the view.
let liveStartHandled = false;

export default class ManLook extends Extension {
    enable() {
        this._injections = new InjectionManager();
        this._injections.overrideMethod(Dialog.Dialog.prototype, 'addButton', original => function (...args) {
            const button = original.apply(this, args);
            this._manAction?.remove_style_class_name('man-action');
            button.add_style_class_name('man-action');
            this._manAction = button;
            return button;
        });
        this._systemMonitorInSystemFolder();
        this._liveSessionStartsOnDesktop();
    }

    // The live session opens MAN Installer at login. GNOME starts a session in the
    // Activities view, where the first click on that window only closes the view,
    // so "Install MAN…" seemed to need two clicks. Only the live session's account:
    // installed systems start as GNOME does.
    _liveSessionStartsOnDesktop() {
        if (liveStartHandled || GLib.get_user_name() !== 'MANInstaller')
            return;
        liveStartHandled = true;
        const leave = () => {
            if (Main.overview.visible)
                Main.overview.hide();
        };
        // Extensions are enabled while the Shell starts, sometimes before its
        // start-up animation decides on the overview, sometimes after.
        if (!Main.layoutManager._startingUp) {
            leave();
            return;
        }
        if (Main.sessionMode.hasOverview) {
            Main.sessionMode.hasOverview = false;
            this._overviewHeldBack = true;
        }
        Main.layoutManager.connectObject('startup-complete', () => {
            Main.layoutManager.disconnectObject(this);
            if (this._overviewHeldBack)
                Main.sessionMode.hasOverview = true;
            this._overviewHeldBack = false;
            leave();
        }, this);
    }

    // The Shell puts its old system monitor in the System folder of a new user's app grid.
    // MAN's is Resources. Once only: afterwards the folder is the user's to change.
    _systemMonitorInSystemFolder() {
        const folder = new Gio.Settings({
            schema_id: 'org.gnome.desktop.app-folders.folder',
            path: '/org/gnome/desktop/app-folders/folders/System/',
        });
        const apps = folder.get_strv('apps');
        if (apps.includes('org.gnome.SystemMonitor.desktop'))
            folder.set_strv('apps', apps.map(a => a === 'org.gnome.SystemMonitor.desktop' ? 'org.gnome.Resources.desktop' : a));
    }

    disable() {
        Main.layoutManager.disconnectObject(this);
        if (this._overviewHeldBack)
            Main.sessionMode.hasOverview = true;
        this._overviewHeldBack = false;
        this._injections.clear();
        this._injections = null;
    }
}
