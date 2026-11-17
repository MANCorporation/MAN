// MAN on Apple Silicon Macs: "Restart to macOS…" in the top bar's power menu, under
// "Restart…". The question and the change are made by Startup Disk
// (man-startup-disk --restart-macos). Only on a Mac started by the MAN boot loader.
import GLib from 'gi://GLib';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const ON_MAC = GLib.file_test('/proc/device-tree/chosen/asahi,efi-system-partition', GLib.FileTest.EXISTS) ||
    GLib.getenv('MAN_FAKE_MAC') === '1';

export default class ManMac extends Extension {
    enable() {
        if (!ON_MAC)
            return;
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
    }
}
