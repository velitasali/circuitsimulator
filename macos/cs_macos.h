/* AppKit helpers. qt-bridge will not hand us a QWindow*, so these attach to
 * NSApp.mainWindow after show. C ABI, compiled from cs-app/build.rs. */

#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*cs_macos_fn)(void);
typedef int (*cs_macos_state_fn)(void);
typedef void (*cs_macos_path_fn)(const char* path);

/* Power: 0 = off (Run), 1 = on (Stop). Pause: 0 = Pause, 1 = Resume, -1 = disabled. */
void cs_macos_install_touchbar(cs_macos_fn power, cs_macos_fn pause, cs_macos_state_fn power_state,
                               cs_macos_state_fn pause_state);
void cs_macos_refresh_touchbar(void);

void cs_macos_init_process(const char* name);
void cs_macos_setup_window(void);
void cs_macos_set_titlebar_dark(int dark);
int cs_macos_apply_theme(const char* theme_name);

void cs_macos_apply_app_icon(const char* path);
void cs_macos_beep(void);

void cs_macos_set_menu_callbacks(cs_macos_path_fn trigger, cs_macos_path_fn about_to_show);
/* JSON array of {title, path, items:[{path,text,enabled,visible,checkable,checked,shortcut,separator,submenu}]} */
void cs_macos_set_menu(const char* json);

#ifdef __cplusplus
}
#endif
