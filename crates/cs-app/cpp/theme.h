/* Cross-platform Qt theme helper (Windows / Linux). macOS uses macos/titlebar.mm. */

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int cs_apply_theme(const char* theme_name);
void cs_set_titlebar_dark(int dark);
void cs_apply_app_icon();
// phase 0: return nonzero when the menu is open. phase 1: toggle, return nonzero
// when that release should be consumed.
void cs_install_alt_menu_filter(int (*on_alt)(int phase));
// Runs a nullary slot now. The queued invoker returns before QML can say
// whether the menu actually toggled.
int cs_invoke_direct(void* obj, const char* method);

#ifdef __cplusplus
}
#endif
