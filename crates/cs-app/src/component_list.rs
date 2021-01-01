//! Component list façade. Rows are a flat JSON array (no `QAbstractItemModel`).

use cs_engine::library::{Library, TYPE_COMPONENT};
use qtbridge::qobject;
use serde_json::{Value, json};

pub struct ComponentList {
    lib: Library,
    search_filter: String,
}

impl Default for ComponentList {
    fn default() -> Self {
        let mut lib = Library::new();
        lib.add_catalog_mcus(&cs_engine::catalog::standard());
        lib.set_recent_components(cs_engine::settings::get().recent_components);
        Self {
            lib,
            search_filter: String::new(),
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl ComponentList {
    qproperty!("rows", Read = rows, Notify = rows_changed);

    #[qsignal]
    fn rows_changed(&mut self);
    #[qsignal]
    fn search_applied(&mut self, filter: String);
    #[qsignal]
    fn drag_started(&mut self, mime: String);

    fn rows(&self) -> Value {
        let arr: Vec<Value> = self
            .lib
            .visible_rows()
            .into_iter()
            .map(|r| {
                let icon_uri = if !r.icon.is_empty() {
                    format!("qrc:/icons/components/{}", r.icon)
                } else {
                    String::new()
                };
                json!({
                    "id": r.id,
                    "caption": r.caption,
                    "display": cs_engine::i18n::tr(&r.caption),
                    "itemType": r.item_type,
                    "compType": r.comp_type,
                    "depth": r.depth,
                    "expanded": r.expanded,
                    "hasChildren": r.has_children,
                    "isCustom": false,
                    "iconUri": icon_uri,
                })
            })
            .collect();
        Value::Array(arr)
    }

    #[qslot]
    fn search(&mut self, filter: String) {
        self.search_filter = filter.clone();
        self.lib.set_filter(&filter);
        self.rows_changed();
        self.search_applied(filter);
    }

    #[qslot]
    fn set_expanded(&mut self, id: u32, expanded: bool) {
        self.lib.set_expanded(id, expanded);
        self.rows_changed();
    }

    #[qslot]
    fn toggle_expanded(&mut self, id: u32) {
        let exp = self.lib.is_expanded(id);
        self.lib.set_expanded(id, !exp);
        self.rows_changed();
    }

    #[qslot]
    fn is_item_expanded(&self, id: u32) -> bool {
        self.lib.is_expanded(id)
    }

    #[qslot]
    fn start_drag(&mut self, id: u32) {
        if let Some(it) = self.lib.item(id) {
            if it.item_type == TYPE_COMPONENT {
                let mime = format!("{},{}", it.caption, it.comp_type);
                self.drag_started(mime);
            }
        }
    }

    #[qslot]
    fn add_recent(&mut self, spec: String) {
        let trimmed = spec.trim();
        if trimmed.is_empty() {
            return;
        }
        cs_engine::settings::edit(|s| s.push_recent_component(trimmed.to_string()));
        self.lib.add_recent(trimmed);
        self.rows_changed();
    }

    #[qslot]
    fn clear_recently_used(&mut self) {
        cs_engine::settings::edit(|s| s.clear_recent_components());
        self.lib.clear_recent();
        self.rows_changed();
    }

    #[qslot]
    fn reload_components(&mut self) {
        let expanded_cats: Vec<(String, bool)> = self
            .lib
            .items()
            .iter()
            .filter(|it| it.item_type != TYPE_COMPONENT)
            .map(|it| (it.caption.clone(), it.expanded))
            .collect();
        let recent_expanded = self.lib.is_expanded(cs_engine::library::RECENT_CATEGORY_ID);

        let mut lib = Library::new();
        lib.add_catalog_mcus(&cs_engine::catalog::standard());
        lib.set_recent_components(cs_engine::settings::get().recent_components);
        lib.set_expanded(cs_engine::library::RECENT_CATEGORY_ID, recent_expanded);

        for (caption, exp) in expanded_cats {
            if let Some(id) = lib
                .items()
                .iter()
                .find(|i| i.caption == caption && i.item_type != TYPE_COMPONENT)
                .map(|i| i.id)
            {
                lib.set_expanded(id, exp);
            }
        }

        if !self.search_filter.is_empty() {
            lib.set_filter(&self.search_filter);
        }
        self.lib = lib;
        self.rows_changed();
    }

    #[qslot]
    fn show_context_menu(&self) {}
}
