// MAN's look for system dialogs: stylesheet.css, and the dialog's action button
// (the last one, as in macOS alerts) marked so that it can be blue.
import Gio from 'gi://Gio';

import * as Dialog from 'resource:///org/gnome/shell/ui/dialog.js';
import {Extension, InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';

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
        this._injections.clear();
        this._injections = null;
    }
}
