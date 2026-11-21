//! Component list façade. Rows are a flat JSON array (no `QAbstractItemModel`).

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use cs_engine::library::{self, Library, RECENT_CATEGORY_ID, TYPE_COMPONENT};
use cs_engine::{catalog, i18n, project, settings};
use qtbridge::qobject;
use serde_json::{Value, json};

use crate::{actions, path_util, pending};

include!(concat!(env!("OUT_DIR"), "/component_svgs.rs"));

static TINTED_ICON_CACHE: LazyLock<Mutex<HashMap<(String, String), String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn svg_hex(color: &str) -> String {
    let hex: String = color.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    // QML color.toString() is "#RRGGBB", or "#AARRGGBB" when alpha is present.
    let rgb = match hex.len() {
        8 => &hex[2..],
        6 => &hex,
        _ => "000000",
    };
    format!("#{rgb}")
}

fn file_url(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    if p.starts_with('/') {
        format!("file://{p}")
    } else {
        format!("file:///{p}")
    }
}

fn icon_file_name(icon_uri: &str) -> Option<&str> {
    let name = icon_uri.rsplit(['/', '\\']).next()?;
    if name.is_empty() || name.contains("..") || !name.to_ascii_lowercase().ends_with(".svg") {
        return None;
    }
    Some(name)
}

fn tinted_icon_file(icon_uri: &str, color: &str) -> String {
    let Some(name) = icon_file_name(icon_uri) else {
        return icon_uri.to_string();
    };
    let Some(svg) = component_svg(name) else {
        return icon_uri.to_string();
    };
    if !svg.contains("currentColor") {
        return icon_uri.to_string();
    }
    let hex = svg_hex(color);
    let tinted = svg.replace("currentColor", &hex);
    let dir = dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("circuitsimulator")
        .join("drag-icons");
    if std::fs::create_dir_all(&dir).is_err() {
        return icon_uri.to_string();
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    tinted.hash(&mut hasher);
    let path = dir.join(format!("{:x}-{name}", hasher.finish()));
    if !path.exists() && std::fs::write(&path, tinted).is_err() {
        return icon_uri.to_string();
    }
    file_url(&path)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {
            let mut out = String::with_capacity(s.len());
            out.push(c.to_ascii_uppercase());
            out.push_str(chars.as_str());
            out
        }
        _ => s.to_string(),
    }
}

fn build_library(circuit_path: &str, project_dir: &str) -> Library {
    let mut lib = Library::new();
    let local = catalog::local_subcircuits(
        (!circuit_path.is_empty()).then_some(circuit_path),
        (!project_dir.is_empty()).then_some(project_dir),
    );
    lib.add_catalog_mcus(&local);
    lib.add_catalog_mcus(&catalog::standard());
    lib.set_recent_components(settings::get().recent_components);
    lib
}

pub struct ComponentList {
    lib: Library,
    search_filter: String,
    circuit_path: String,
    project_dir: String,
}

impl Default for ComponentList {
    fn default() -> Self {
        let project_dir = if pending::no_project() {
            String::new()
        } else {
            let s = settings::get();
            s.last_project_dir
                .or_else(|| s.recent_projects.first().cloned())
                .map(|p| project::normalize_path(&p))
                .unwrap_or_default()
        };
        let lib = build_library("", &project_dir);
        Self {
            lib,
            search_filter: String::new(),
            circuit_path: String::new(),
            project_dir,
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
        let custom_shortcuts = settings::with(|s| s.shortcuts.clone());
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
                let (spec, action_id, shortcut, default_shortcut) = if r.item_type == TYPE_COMPONENT
                {
                    let spec = library::canonical_component_spec(&r.caption, &r.comp_type);
                    let action_id = actions::component_action_id(&spec);
                    let shortcut = actions::effective_shortcut(&action_id, &custom_shortcuts);
                    let default_shortcut = actions::default_shortcut(&action_id).to_string();
                    (spec, action_id, shortcut, default_shortcut)
                } else {
                    (String::new(), String::new(), String::new(), String::new())
                };
                json!({
                    "id": r.id,
                    "caption": r.caption,
                    "display": i18n::tr(&r.caption),
                    "itemType": r.item_type,
                    "compType": r.comp_type,
                    "spec": spec,
                    "actionId": action_id,
                    "shortcut": shortcut,
                    "defaultShortcut": default_shortcut,
                    "depth": r.depth,
                    "expanded": r.expanded,
                    "hasChildren": r.has_children,
                    "isCustom": r.is_custom,
                    "iconUri": icon_uri,
                })
            })
            .collect();
        Value::Array(arr)
    }

    #[qslot]
    fn refresh_shortcuts(&mut self) {
        self.rows_changed();
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
    fn component_info(&self, id: u32) -> Value {
        let Some(it) = self.lib.item(id) else {
            return Value::Null;
        };
        if it.item_type != TYPE_COMPONENT {
            return Value::Null;
        }
        let icon_uri = if !it.icon.is_empty() {
            format!("qrc:/icons/components/{}", it.icon)
        } else {
            String::new()
        };
        let description = it
            .description()
            .map(|d| capitalize_first(&i18n::tr(d)))
            .unwrap_or_default();
        let category = self
            .lib
            .category_path(id)
            .into_iter()
            .map(i18n::tr)
            .collect::<Vec<_>>()
            .join(" › ");
        let spec = library::canonical_component_spec(&it.caption, &it.comp_type);
        let action_id = actions::component_action_id(&spec);
        let shortcut = settings::with(|s| actions::effective_shortcut(&action_id, &s.shortcuts));
        json!({
            "id": id,
            "title": i18n::tr(&it.caption),
            "description": description,
            "category": category,
            "compType": it.comp_type,
            "spec": spec,
            "shortcut": shortcut,
            "iconUri": icon_uri,
            "isCustom": it.is_custom,
        })
    }

    /// File URL of `icon_uri` with `currentColor` rewritten to `color`.
    /// Drag.imageSource loads this the same way as the original qrc SVG.
    #[qslot]
    fn tinted_icon_url(&self, icon_uri: String, color: String) -> String {
        let key = (icon_uri.clone(), color.clone());
        if let Ok(cache) = TINTED_ICON_CACHE.lock()
            && let Some(url) = cache.get(&key)
        {
            return url.clone();
        }
        let url = tinted_icon_file(&icon_uri, &color);
        if let Ok(mut cache) = TINTED_ICON_CACHE.lock() {
            cache.insert(key, url.clone());
        }
        url
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
        let canon = library::normalize_component_spec(&spec);
        if canon.is_empty() {
            return;
        }
        settings::edit(|s| {
            s.recent_components
                .retain(|existing| library::normalize_component_spec(existing) != canon);
            s.push_recent_component(canon.clone());
        });
        self.lib.add_recent(&canon);
        self.rows_changed();
    }

    #[qslot]
    fn clear_recently_used(&mut self) {
        cs_engine::settings::edit(|s| s.clear_recent_components());
        self.lib.clear_recent();
        self.rows_changed();
    }

    #[qslot]
    fn set_circuit_path(&mut self, path: String) {
        let clean = path_util::strip_file_url(&path);
        if self.circuit_path != clean {
            self.circuit_path = clean;
            self.reload_components();
        }
    }

    #[qslot]
    fn set_project_dir(&mut self, dir: String) {
        let clean = project::normalize_path(&path_util::strip_file_url(&dir));
        if self.project_dir != clean {
            self.project_dir = clean;
            self.reload_components();
        }
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
        let recent_expanded = self.lib.is_expanded(RECENT_CATEGORY_ID);

        let mut lib = build_library(&self.circuit_path, &self.project_dir);
        lib.set_expanded(RECENT_CATEGORY_ID, recent_expanded);

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
