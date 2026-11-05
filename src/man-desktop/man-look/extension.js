// MAN's look for system dialogs: stylesheet.css, and the dialog's action button
// (the last one, as in macOS alerts) marked so that it can be blue.
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
    }

    disable() {
        this._injections.clear();
        this._injections = null;
    }
}
