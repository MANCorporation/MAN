// MAN's dock. On the desktop it is a second copy of the Shell's own dash, so it has the same
// icons, menus and look. In the Activities view it steps aside and nothing there is changed:
// what shows is the Shell's dash, in the Shell's place.
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import St from 'gi://St';

import * as Dash from 'resource:///org/gnome/shell/ui/dash.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension, InjectionManager} from 'resource:///org/gnome/shell/extensions/extension.js';

const DEFAULT_ICON_SIZE = 64;
const SHELL_MAX_HEIGHT_RATIO = 0.16; // DASH_MAX_HEIGHT_RATIO of overviewControls.js
const SLIDE_TIME = 200;
const FADE_TIME = 150;
const HIDE_DELAY = 400;

const DockDash = GObject.registerClass(
class DockDash extends Dash.Dash {
    _init(iconSize, onMenu) {
        this._wantedIconSize = iconSize;
        this._onMenu = onMenu;
        super._init();
        Main.ctrlAltTabManager.removeGroup(this);
    }

    _itemMenuStateChanged(item, opened) {
        super._itemMenuStateChanged(item, opened);
        this._onMenu(opened);
    }

    // The default size is whatever the Shell would choose; another one is taken as it is,
    // made smaller only when the icons would not fit on the screen.
    _adjustIconSize() {
        if (this._wantedIconSize === DEFAULT_ICON_SIZE) {
            super._adjustIconSize();
            return;
        }
        if (this._maxWidth === -1)
            return;
        const icons = this._box.get_children()
            .filter(a => a.child?._delegate?.icon && !a.animatingOut);
        icons.push(this._showAppsIcon);
        const scale = St.ThemeContext.get_for_stage(global.stage).scale_factor;
        const room = Math.floor(this._maxWidth / scale / icons.length) - 24;
        const size = Math.max(16, Math.min(this._wantedIconSize, room));
        if (size === this.iconSize)
            return;
        this.iconSize = size;
        this.emit('icon-size-changed');
        icons.forEach(a => a.child._delegate.icon.setIconSize(size));
    }
});

class Dock {
    constructor(monitor, settings) {
        this._monitor = monitor;
        this._fixed = settings.get_boolean('dock-fixed');
        this._menuOpen = false;
        this._revealed = false;
        this._shown = false;

        this._dash = new DockDash(settings.get_int('dash-max-icon-size'), opened => {
            this._menuOpen = opened;
            this._queueUpdate();
        });
        this._dash.add_style_class_name('man-dock');
        this._dash.set({reactive: true, track_hover: true, visible: false, opacity: 0});
        this._dash.setMaxSize(monitor.width, Math.round(monitor.height * SHELL_MAX_HEIGHT_RATIO));
        Main.layoutManager.addChrome(this._dash, {affectsStruts: this._fixed});

        // The strip at the screen's edge that brings a hidden dock back.
        this._edge = new St.Widget({reactive: true, track_hover: true, visible: false});
        Main.layoutManager.addChrome(this._edge);
        this._edge.set_position(monitor.x, monitor.y + monitor.height - 1);
        this._edge.set_size(monitor.width, 1);

        this._dash.connect('notify::width', () => this._place());
        this._dash.connect('notify::height', () => this._place());
        this._dash.connect('notify::hover', () => this._pointerChanged());
        this._edge.connect('notify::hover', () => this._pointerChanged());
        this._dash.showAppsButton.connect('notify::checked', button => {
            if (button.checked) {
                button.checked = false;
                Main.overview.showApps();
            }
        });

        Main.overview.connectObject(
            'showing', () => this._update(),
            'hiding', () => this._update(),
            'hidden', () => this._update(),
            this._dash);
        global.display.connectObject(
            'window-created', (_, window) => this._watch(window),
            'restacked', () => this._queueUpdate(),
            'in-fullscreen-changed', () => this._queueUpdate(),
            this._dash);
        global.workspace_manager.connectObject(
            'active-workspace-changed', () => this._queueUpdate(), this._dash);
        global.get_window_actors().forEach(a => this._watch(a.meta_window));

        this._place();
        this._update();
    }

    destroy() {
        for (const id of [this._updateId, this._hideId]) {
            if (id)
                GLib.source_remove(id);
        }
        this._edge.destroy();
        this._dash.destroy();
    }

    _watch(window) {
        window.connectObject(
            'position-changed', () => this._queueUpdate(),
            'size-changed', () => this._queueUpdate(),
            'workspace-changed', () => this._queueUpdate(),
            'notify::minimized', () => this._queueUpdate(),
            'unmanaged', () => this._queueUpdate(),
            this._dash);
        this._queueUpdate();
    }

    _place() {
        const {x, y, width, height} = this._monitor;
        const [dockWidth, dockHeight] = [this._dash.width, this._dash.height];
        this._dash.set_position(x + Math.floor((width - dockWidth) / 2), y + height - dockHeight);
        this._queueUpdate();
    }

    _pointerChanged() {
        if (this._edge.hover || this._dash.hover) {
            if (this._hideId) {
                GLib.source_remove(this._hideId);
                this._hideId = 0;
            }
            if (this._edge.hover)
                this._revealed = true;
            this._update();
        } else if (this._revealed && !this._hideId) {
            this._hideId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, HIDE_DELAY, () => {
                this._hideId = 0;
                this._revealed = false;
                this._update();
                return GLib.SOURCE_REMOVE;
            });
        }
    }

    _queueUpdate() {
        this._updateId ||= GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._updateId = 0;
            this._update();
            return GLib.SOURCE_REMOVE;
        });
    }

    _windowInTheWay() {
        const top = this._monitor.y + this._monitor.height - this._dash.height;
        const left = this._dash.x;
        const right = left + this._dash.width;
        const workspace = global.workspace_manager.get_active_workspace();
        return global.get_window_actors().some(({meta_window: window}) => {
            if (window.minimized || window.is_override_redirect() || window.is_skip_taskbar() ||
                window.get_window_type() === Meta.WindowType.DESKTOP ||
                !window.located_on_workspace(workspace))
                return false;
            const frame = window.get_frame_rect();
            return frame.y + frame.height > top && frame.x < right && frame.x + frame.width > left;
        });
    }

    _update() {
        if (Main.overview.visibleTarget) {
            this._edge.hide();
            this._leave(true);
            return;
        }
        if (Main.overview.visible && !Main.overview.animationInProgress)
            return;
        if (this._monitor.inFullscreen) {
            this._edge.hide();
            this._leave(false);
            return;
        }
        const show = this._fixed || this._revealed || this._menuOpen || this._dash.hover ||
            !this._windowInTheWay();
        if (show)
            this._enter();
        else
            this._leave(false);
        this._edge.visible = !this._shown;
    }

    _enter() {
        if (this._shown)
            return;
        this._shown = true;
        const leavingOverview = Main.overview.visible;
        this._dash.remove_all_transitions();
        this._dash.set({visible: true, translation_y: leavingOverview ? 0 : this._dash.height});
        this._dash.ease({
            translation_y: 0,
            opacity: 255,
            duration: leavingOverview ? FADE_TIME : SLIDE_TIME,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
        });
    }

    // Into the Activities view it fades, as the Shell's dash takes the same place.
    _leave(fade) {
        if (!this._shown)
            return;
        this._shown = false;
        this._dash.remove_all_transitions();
        this._dash.ease({
            translation_y: fade ? 0 : this._dash.height,
            opacity: fade ? 0 : 255,
            duration: fade ? FADE_TIME : SLIDE_TIME,
            mode: Clutter.AnimationMode.EASE_OUT_QUAD,
            onStopped: finished => {
                if (finished)
                    this._dash.set({visible: false, opacity: 0});
            },
        });
    }
}

export default class ManDock extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._docks = [];

        // Windows keep clear of an always-shown dock, but the Activities view stays as it is
        // without one: it lays itself out as if only the top bar took room.
        const layout = Main.overview._overview.controls.layout_manager;
        this._injections = new InjectionManager();
        this._injections.overrideMethod(layout.constructor.prototype, '_updateWorkAreaBox', () => function () {
            const monitor = Main.layoutManager.primaryMonitor;
            if (!monitor)
                return;
            const top = Main.layoutManager.getWorkAreaForMonitor(monitor.index).y - monitor.y;
            this._workAreaBox.set_origin(0, top);
            this._workAreaBox.set_size(monitor.width, monitor.height - top);
        });
        layout._updateWorkAreaBox();

        this._settings.connectObject('changed', () => this._rebuild(), this);
        Main.layoutManager.connectObject('monitors-changed', () => this._rebuild(), this);
        if (Main.layoutManager._startingUp)
            Main.layoutManager.connectObject('startup-complete', () => this._rebuild(), this);
        else
            this._rebuild();
    }

    disable() {
        this._settings.disconnectObject(this);
        Main.layoutManager.disconnectObject(this);
        this._docks.forEach(d => d.destroy());
        this._docks = null;
        this._injections.clear();
        this._injections = null;
        Main.overview._overview.controls.layout_manager._updateWorkAreaBox();
        this._settings = null;
    }

    _rebuild() {
        this._docks.forEach(d => d.destroy());
        const {monitors, primaryMonitor} = Main.layoutManager;
        const places = this._settings.get_boolean('multi-monitor') ? monitors : [primaryMonitor];
        this._docks = places.filter(Boolean).map(m => new Dock(m, this._settings));
    }
}
