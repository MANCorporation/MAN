// MAN on Apple Silicon Macs: "Restart to macOS…" in the top bar's power menu, under
// "Restart…", and DUHACH! in the top bar (always there, from the start of the
// session): Restart to macOS… / More Options… The question and the changes are made
// by man-startup-disk (the DUHACH! window). Only on a Mac started by the MAN boot loader.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

// Set when DUHACH! deleted macOS: nothing to restart to.
const MACOS_DELETED = '/var/lib/man/macos-deleted';

const ON_MAC = GLib.file_test('/proc/device-tree/chosen/asahi,efi-system-partition', GLib.FileTest.EXISTS) ||
    GLib.getenv('MAN_FAKE_MAC') === '1';

export default class ManMac extends Extension {
    enable() {
        if (!ON_MAC)
            return;
        this._addIndicator();
        this._tries = 0;
        // The quick settings' system item is created after the extensions load.
        this._source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
            if (this._addItem() || ++this._tries > 40) {
                this._source = 0;
                return GLib.SOURCE_REMOVE;
            }
            return GLib.SOURCE_CONTINUE;
        });
    }

    _addIndicator() {
        this._indicator = new PanelMenu.Button(0.0, 'DUHACH!', false);
        this._indicator.add_child(new St.Icon({
            gicon: Gio.ThemedIcon.new('com.mancorporation.Duhach-symbolic'),
            style_class: 'system-status-icon',
        }));
        const menu = this._indicator.menu;
        const title = new PopupMenu.PopupMenuItem('DUHACH!', {reactive: false});
        title.label.style = 'font-weight: bold;';
        menu.addMenuItem(title);
        this._restartItem = menu.addAction('Restart to macOS…',
            () => GLib.spawn_command_line_async('man-startup-disk --restart-macos'));
        menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        menu.addAction('More Options…', () => GLib.spawn_command_line_async('man-startup-disk'));
        // Each time it opens: no "Restart to macOS" once macOS is gone.
        menu.connect('open-state-changed', (_m, open) => {
            if (open)
                this._restartItem.visible = !GLib.file_test(MACOS_DELETED, GLib.FileTest.EXISTS);
        });
        Main.panel.addToStatusArea('man-duhach', this._indicator, 0, 'right');
    }

    _shutdownMenu() {
        // GNOME 51 status/system.js: SystemItem.menu is its ShutdownItem's menu
        // (Suspend, Restart…, Power Off…, Log Out, Switch User…).
        const system = Main.panel.statusArea.quickSettings?._system;
        return system?.quickSettingsItems?.[0]?.menu ?? null;
    }

    _addItem() {
        const menu = this._shutdownMenu();
        if (!menu)
            return false;
        if (GLib.file_test(MACOS_DELETED, GLib.FileTest.EXISTS))
            return true;
        this._item = new PopupMenu.PopupMenuItem('Restart to macOS…');
        this._item.connect('activate', () => {
            Main.panel.closeQuickSettings();
            GLib.spawn_command_line_async('man-startup-disk --restart-macos');
        });
        // Right after "Restart…": the menu is Suspend, Restart…, Power Off…, separator,
        // Log Out…, Switch User… (hidden ones stay in the list, so 2 is always right).
        menu.addMenuItem(this._item, 2);
        return true;
    }

    disable() {
        if (this._source) {
            GLib.source_remove(this._source);
            this._source = 0;
        }
        this._item?.destroy();
        this._item = null;
        this._indicator?.destroy();
        this._indicator = null;
    }
}
