/* Cross-platform Qt theme helper and non-macOS window event filters. */

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct CsQtPalette {
    uint32_t window;
    uint32_t window_text;
    uint32_t base;
    uint32_t alternate_base;
    uint32_t tooltip_base;
    uint32_t tooltip_text;
    uint32_t text;
    uint32_t placeholder_text;
    uint32_t disabled_text;
    uint32_t disabled_window_text;
    uint32_t button;
    uint32_t button_text;
    uint32_t disabled_button_text;
    uint32_t bright_text;
    uint32_t link;
    uint32_t highlight;
    uint32_t highlighted_text;
    uint32_t accent;
    uint32_t mid;
    uint32_t midlight;
    uint32_t shadow;
    uint32_t dark;
    uint32_t light;
} CsQtPalette;

/* Returns -2 if no QGuiApplication, -1 if Unknown, 0 = Light, 1 = Dark. */
int cs_qt_query_system_dark(int unset_scheme);
/* scheme_mode: 0 = System (leave unset), 1 = Light, 2 = Dark. */
void cs_qt_apply_palette(int scheme_mode, const CsQtPalette* pal);
/* Posts `cb` to run on the Qt main event loop (or runs immediately if no QCoreApplication). */
void cs_qt_post_on_main(void (*cb)());
/* Moves a UTF-8 path (file or directory) to the OS Trash / Recycle Bin via QFile::moveToTrash.
 * Returns 1 on success, 0 on failure. */
int cs_qt_move_to_trash(const uint8_t* path_utf8, uintptr_t len);
/* Opens a local file or directory in the OS desktop shell / default application
 * via QDesktopServices::openUrl(QUrl::fromLocalFile(path)).
 * Returns 1 on success, 0 on failure. */
int cs_qt_open_local_path(const uint8_t* path_utf8, uintptr_t len);

void cs_apply_app_icon();
void cs_ensure_theme_filter();
void cs_set_titlebar_dark(int dark);
/* phase 0: return nonzero when the menu is open. phase 1: toggle, return nonzero
 * when that release should be consumed. */
void cs_install_alt_menu_filter(int (*on_alt)(int phase));
/* Runs a nullary slot now. The queued invoker returns before QML can say
 * whether the menu actually toggled. */
int cs_invoke_direct(void* obj, const char* method);

#ifdef __cplusplus
}
#endif
