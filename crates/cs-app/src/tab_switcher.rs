//! Tab Switcher / Recent tabs and circuits cycling popup.

use qtbridge::qobject;
use serde_json::Value;

pub struct TabSwitcher {
    entries: Vec<Value>,
    current_row: i32,
    visible: bool,
}

impl Default for TabSwitcher {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            current_row: 0,
            visible: false,
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl TabSwitcher {
    qproperty!("entries", Read = entries, Notify = entries_changed);
    qproperty!(
        "currentRow",
        Read = current_row,
        Write = set_current_row,
        Notify = current_row_changed
    );
    qproperty!(
        "visible",
        Read = is_visible,
        Write = set_visible,
        Notify = visible_changed
    );

    #[qsignal]
    fn entries_changed(&mut self);
    #[qsignal]
    fn current_row_changed(&mut self);
    #[qsignal]
    fn visible_changed(&mut self);
    #[qsignal]
    fn execute_item(&mut self, kind: String, id: String, index: i32);

    fn entries(&self) -> Value {
        Value::Array(self.entries.clone())
    }

    fn current_row(&self) -> i32 {
        self.current_row
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, v: bool) {
        if self.visible != v {
            self.visible = v;
            self.visible_changed();
        }
    }

    fn set_current_row(&mut self, row: i32) {
        let clamped = if self.entries.is_empty() {
            -1
        } else {
            row.clamp(0, self.entries.len() as i32 - 1)
        };
        if self.current_row != clamped {
            self.current_row = clamped;
            self.current_row_changed();
        }
    }

    #[qslot]
    fn set_entries_json(&mut self, items: String) {
        if let Ok(Value::Array(arr)) = serde_json::from_str(&items) {
            self.entries = arr;
            self.entries_changed();
        }
    }

    #[qslot]
    fn populate(&mut self, items: Value) {
        if let Value::Array(arr) = items {
            self.entries = arr;
            self.entries_changed();
        }
    }

    #[qslot]
    fn open_switcher(&mut self, forward: bool) {
        if self.visible {
            self.move_selection(if forward { 1 } else { -1 });
            return;
        }
        if self.entries.is_empty() {
            return;
        }
        let count = self.entries.len() as i32;
        let next_row = if count >= 2 {
            if forward { 1 } else { count - 1 }
        } else {
            0
        };
        self.set_current_row(next_row);
        self.set_visible(true);
    }

    #[qslot]
    fn move_selection(&mut self, delta: i32) {
        if self.entries.is_empty() {
            self.set_current_row(-1);
            return;
        }
        let len = self.entries.len() as i32;
        let next = (self.current_row + delta).rem_euclid(len);
        self.set_current_row(next);
    }

    #[qslot]
    fn accept(&mut self) {
        self.set_visible(false);
    }

    #[qslot]
    fn reject(&mut self) {
        self.set_visible(false);
    }

    #[qslot]
    fn commit_selection(&mut self) {
        if self.current_row < 0 || self.current_row as usize >= self.entries.len() {
            self.set_visible(false);
            return;
        }
        let item = self.entries[self.current_row as usize].clone();
        let kind = item["kind"].as_str().unwrap_or_default().to_string();
        let id = item["id"].as_str().unwrap_or_default().to_string();
        let index = item["index"].as_i64().unwrap_or(-1) as i32;
        self.set_visible(false);
        self.execute_item(kind, id, index);
    }
}
