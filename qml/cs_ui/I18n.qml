pragma Singleton

import QtQuick
import cs_app

QtObject {
    function tr(s: string): string {
        return App.i18nTick >= 0 ? App.translate(s) : s
    }
}
