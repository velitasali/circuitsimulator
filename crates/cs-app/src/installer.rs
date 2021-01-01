//! Component Library Installer bridge for Qt Quick.

use cs_engine::installer::InstallerManager;
use qtbridge::qobject;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

pub struct Installer {
    manager: Arc<Mutex<InstallerManager>>,
    info: Value,
    status_text: String,
    busy: bool,
    busy_item: String,
}

impl Default for Installer {
    fn default() -> Self {
        Self {
            manager: Arc::new(Mutex::new(InstallerManager::new())),
            info: json!({
                "visible": false,
                "title": "",
                "description": "",
                "author": "",
                "items": [],
            }),
            status_text: String::new(),
            busy: false,
            busy_item: String::new(),
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl Installer {
    qproperty!("items", Read = items, Notify = items_changed);
    qproperty!("info", Read = info, Notify = info_changed);
    qproperty!(
        "statusText",
        Read = status_text,
        Notify = status_text_changed
    );
    qproperty!("busy", Read = busy, Notify = busy_changed);

    fn status_text(&self) -> String {
        self.status_text.clone()
    }

    fn busy(&self) -> bool {
        self.busy
    }

    #[qsignal]
    fn items_changed(&mut self);
    #[qsignal]
    fn info_changed(&mut self);
    #[qsignal]
    fn status_text_changed(&mut self);
    #[qsignal]
    fn busy_changed(&mut self);
    #[qsignal]
    fn package_installed(&mut self, name: String);
    #[qsignal]
    fn package_uninstalled(&mut self, name: String);

    fn items(&self) -> Value {
        let mgr = self.manager.lock().unwrap();
        let list: Vec<Value> = mgr
            .items()
            .iter()
            .map(|item| {
                json!({
                    "name": item.name,
                    "description": item.description,
                    "author": item.author,
                    "isGroupHeader": item.is_group_header(),
                    "installed": item.installed(),
                    "canUpdate": item.can_update(),
                    "busy": self.busy_item == item.name,
                })
            })
            .collect();
        Value::Array(list)
    }

    fn info(&self) -> Value {
        self.info.clone()
    }

    #[qslot]
    fn check_for_updates_clicked(&mut self) {
        self.busy = true;
        self.status_text = cs_engine::i18n::tr("Checking for updates...");
        self.busy_changed();
        self.status_text_changed();

        let res = self.manager.lock().unwrap().check_for_updates();
        self.busy = false;
        match res {
            Ok(has_updates) => {
                self.status_text = if has_updates {
                    cs_engine::i18n::tr("Updates are available.")
                } else {
                    cs_engine::i18n::tr("All component sets are up to date.")
                };
            }
            Err(e) => {
                let err_prefix = cs_engine::i18n::tr("Error checking updates:");
                self.status_text = format!("{err_prefix} {e}");
            }
        }
        self.items_changed();
        self.busy_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn toggle_install(&mut self, item_name: String) {
        let item_opt = self.manager.lock().unwrap().item(&item_name).cloned();
        let Some(item) = item_opt else {
            return;
        };
        if item.is_group_header() {
            return;
        }

        self.busy = true;
        self.busy_item = item_name.clone();
        self.busy_changed();
        self.items_changed();

        if item.installed() && !item.can_update() {
            // Uninstall
            self.status_text = cs_engine::i18n::tr("Uninstalling %1...").replace("%1", &item.name);
            self.status_text_changed();
            let res = self.manager.lock().unwrap().uninstall_set(&item.name);
            match res {
                Ok(()) => {
                    self.status_text =
                        cs_engine::i18n::tr("Uninstalled %1.").replace("%1", &item.name);
                    self.package_uninstalled(item.name.clone());
                }
                Err(e) => {
                    let err_prefix = cs_engine::i18n::tr("Uninstall error:");
                    self.status_text = format!("{err_prefix} {e}");
                }
            }
        } else {
            // Install or Update
            self.status_text = cs_engine::i18n::tr("Installing %1...").replace("%1", &item.name);
            self.status_text_changed();
            let res = self.manager.lock().unwrap().install_set(&item.name);
            match res {
                Ok(()) => {
                    self.status_text =
                        cs_engine::i18n::tr("Successfully installed %1.").replace("%1", &item.name);
                    self.package_installed(item.name.clone());
                }
                Err(e) => {
                    let err_prefix = cs_engine::i18n::tr("Installation error:");
                    self.status_text = format!("{err_prefix} {e}");
                }
            }
        }

        self.busy = false;
        self.busy_item.clear();
        self.busy_changed();
        self.items_changed();
        self.status_text_changed();
    }

    #[qslot]
    fn show_info(&mut self, item_name: String) {
        let (title, description, author, group_items) = {
            let mgr = self.manager.lock().unwrap();
            if let Some(item) = mgr.item(&item_name) {
                let group_items = mgr.get_group_items(&item_name);
                (
                    item.name.clone(),
                    item.description.clone(),
                    item.author.clone(),
                    group_items,
                )
            } else {
                return;
            }
        };
        self.info = json!({
            "visible": true,
            "title": title,
            "description": description,
            "author": author,
            "items": group_items,
        });
        self.info_changed();
    }

    #[qslot]
    fn clear_info(&mut self) {
        self.info = json!({
            "visible": false,
            "title": "",
            "description": "",
            "author": "",
            "items": [],
        });
        self.info_changed();
    }
}
