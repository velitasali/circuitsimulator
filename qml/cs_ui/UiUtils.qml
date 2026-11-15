pragma Singleton

import QtQuick
import cs_app

QtObject {
    readonly property list<real> timeDivSteps: [
        1e-9, 1.2e-9, 1.5e-9, 2e-9, 2.5e-9, 3e-9, 4e-9, 5e-9, 6e-9, 7e-9, 8e-9, 9e-9,
        1e-8, 1.2e-8, 1.5e-8, 2e-8, 2.5e-8, 3e-8, 4e-8, 5e-8, 6e-8, 7e-8, 8e-8, 9e-8,
        1e-7, 1.2e-7, 1.5e-7, 2e-7, 2.5e-7, 3e-7, 4e-7, 5e-7, 6e-7, 7e-7, 8e-7, 9e-7,
        1e-6, 1.2e-6, 1.5e-6, 2e-6, 2.5e-6, 3e-6, 4e-6, 5e-6, 6e-6, 7e-6, 8e-6, 9e-6,
        1e-5, 1.2e-5, 1.5e-5, 2e-5, 2.5e-5, 3e-5, 4e-5, 5e-5, 6e-5, 7e-5, 8e-5, 9e-5,
        1e-4, 1.2e-4, 1.5e-4, 2e-4, 2.5e-4, 3e-4, 4e-4, 5e-4, 6e-4, 7e-4, 8e-4, 9e-4,
        1e-3, 1.2e-3, 1.5e-3, 2e-3, 2.5e-3, 3e-3, 4e-3, 5e-3, 6e-3, 7e-3, 8e-3, 9e-3,
        1e-2, 1.2e-2, 1.5e-2, 2e-2, 2.5e-2, 3e-2, 4e-2, 5e-2, 6e-2, 7e-2, 8e-2, 9e-2,
        0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9,
        1.0, 1.2, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0
    ]

    function isLightColor(c: color): bool {
        var lum = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b
        return lum > 0.55
    }

    function formatEng(val: real, unit: string): string {
        if (val === 0) return "0 " + unit
        var abs = Math.abs(val)
        if (abs < 1e-9) return (val * 1e12).toFixed(1) + " p" + unit
        if (abs < 1e-6) return (val * 1e9).toFixed(1) + " n" + unit
        if (abs < 1e-3) return (val * 1e6).toFixed(1) + " µ" + unit
        if (abs < 1.0) return (val * 1e3).toFixed(1) + " m" + unit
        if (abs >= 1e9) return (val / 1e9).toFixed(2) + " G" + unit
        if (abs >= 1e6) return (val / 1e6).toFixed(2) + " M" + unit
        if (abs >= 1e3) return (val / 1e3).toFixed(2) + " k" + unit
        return val.toFixed(2) + " " + unit
    }

    function formatEngTime(val: real): string {
        var sign = val < 0 ? "-" : "+"
        var abs = Math.abs(val)
        if (abs < 1e-9) return "0.00 s"
        if (abs < 1e-6) return sign + (abs * 1e9).toFixed(1) + " ns"
        if (abs < 1e-3) return sign + (abs * 1e6).toFixed(1) + " µs"
        if (abs < 1.0) return sign + (abs * 1e3).toFixed(2) + " ms"
        return sign + abs.toFixed(2) + " s"
    }

    function stepLogList(steps: list<real>, cur: real, dir: int): real {
        if (steps.length === 0)
            return cur
        // Fallback when cur is not finite: 1 ms in timeDivSteps, clamped for shorter lists.
        var idx = Math.min(18, steps.length - 1)
        var minDiff = 1e12
        for (var i = 0; i < steps.length; ++i) {
            var diff = Math.abs(Math.log(Math.max(1e-18, steps[i])) - Math.log(Math.max(1e-18, cur)))
            if (diff < minDiff) {
                minDiff = diff
                idx = i
            }
        }
        if (dir > 0) {
            if (steps[idx] <= cur * 1.0001 && idx + 1 < steps.length)
                idx = idx + 1
        } else if (dir < 0) {
            if (steps[idx] >= cur * 0.9999 && idx - 1 >= 0)
                idx = idx - 1
        }
        var nextIdx = Math.max(0, Math.min(steps.length - 1, idx))
        return steps[nextIdx]
    }

    function readNumber(text: string, fallback: real): real {
        var v = Number(text)
        return isFinite(v) ? v : fallback
    }

    function shortcutFor(actionId: string): string {
        var scs = AppMenuBar.shortcuts
        if (!scs) return ""
        for (var i = 0; i < scs.length; ++i) {
            if (scs[i] && scs[i].id === actionId)
                return scs[i].shortcut || ""
        }
        return ""
    }
}
