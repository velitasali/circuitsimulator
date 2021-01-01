/* Cross-platform Qt theme helper (Windows / Linux). macOS uses macos/titlebar.mm. */

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

int cs_apply_theme(const char* theme_name);
void cs_set_titlebar_dark(int dark);
void cs_apply_app_icon();

#ifdef __cplusplus
}
#endif
