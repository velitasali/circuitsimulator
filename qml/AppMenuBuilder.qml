pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls
import cs_app
import cs_ui

/* Shared builder that populates AppContextMenu trees from AppMenuBar JSON. */
Item {
    id: builder
    visible: false

    Component {
        id: menuItemComp
        ContextMenuItem {
            id: menuLeafItem
            // Path into the AppMenuBar JSON; "" for separators/parents.
            property string itemPath: ""
            onTriggered: {
                if (menuLeafItem.itemPath !== "")
                    AppMenuBar.trigger(menuLeafItem.itemPath)
            }
        }
    }

    Component {
        id: subMenuComp
        AppContextMenu {
            popupType: Popup.Item
            focus: true
        }
    }

    function clearMenu(menu: AppContextMenu): void {
        for (var i = menu.count - 1; i >= 0; --i) {
            var sub = menu.menuAt(i) as AppContextMenu
            if (sub) {
                // Leaves are QObject-parented to the builder, so destroying the
                // submenu does not delete them.
                builder.clearMenu(sub)
                var taken = menu.takeMenu(i)
                if (taken)
                    taken.destroy()
            } else {
                var item = menu.takeItem(i)
                if (item)
                    item.destroy()
            }
        }
    }

    function makeLeaf(entry: var): ContextMenuItem {
        var isSep = !!entry.separator
        // Parent rows can never fire: submenu entries carry no action id
        // and AppMenuBar.trigger ignores them, so itemPath stays "" there.
        // Parent temporarily to `builder` (in the scene), not to the Menu (a Popup),
        // before addItem() reparents into the menu's visual hierarchy.
        return menuItemComp.createObject(builder, {
            text: isSep ? "" : UiUtils.stripMnemonic(String(entry.text)),
            enabled: !isSep && !!entry.enabled,
            visible: entry.visible !== false,
            isSeparator: isSep,
            checkable: !isSep && !!entry.checkable && !entry.submenu,
            checked: !!entry.checked,
            shortcutText: entry.shortcut ? UiUtils.formatShortcut(String(entry.shortcut)) : "",
            itemPath: (!isSep && !entry.submenu && entry.path) ? String(entry.path) : ""
        }) as ContextMenuItem
    }

    function populateMenu(menu: AppContextMenu, items: var): void {
        menu.populating = true
        builder.clearMenu(menu)
        if (!items) {
            menu.populating = false
            menu.updateGeometry()
            return
        }
        for (var i = 0; i < items.length; ++i) {
            var entry = items[i]
            if (!entry)
                continue
            if (entry.submenu) {
                if (entry.visible === false)
                    continue
                var sub = subMenuComp.createObject(menu, {
                    title: UiUtils.stripMnemonic(String(entry.text))
                }) as AppContextMenu
                if (!sub)
                    continue
                builder.populateMenu(sub, entry.submenu)
                menu.addMenu(sub)
            } else {
                var leaf = builder.makeLeaf(entry)
                if (leaf)
                    menu.addItem(leaf)
            }
        }
        menu.populating = false
        menu.updateGeometry()
    }

    function createSubMenu(parentMenu: AppContextMenu, titleText: string): AppContextMenu {
        return subMenuComp.createObject(parentMenu, {
            title: UiUtils.stripMnemonic(titleText)
        }) as AppContextMenu
    }
}
