//! Mutation result merged into canvas [`crate::canvas::Change`].

/// How the QML view should refresh after a component mutation.
///
/// Pixel invalidation is `DirtySet` (rasters_item). `Change.items` is the
/// QML `CircuitCanvas.items` JSON model and must not be set for knobs,
/// live holds, or sim paint — that serializes every component on each event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewUpdate {
    None,
    /// Dirty-region raster only. Knobs, live holds, high-frequency values.
    Raster,
    /// Rebuild the QML items model (place/delete, selection, structural, dialog).
    #[default]
    ReplaceItem,
    /// Same raster-only contract as [`Self::Raster`].
    Fields(&'static [&'static str]),
}

impl ViewUpdate {
    pub fn rasters_item(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn rebuilds_qml_item(self) -> bool {
        matches!(self, Self::ReplaceItem)
    }

    pub fn patches_item(self) -> bool {
        self.rasters_item()
    }
}

/// Flags for one component mutation. Canvas maps these onto `items` / `props`
/// / `history` (QML `modified`) / sim patch so callers cannot drop a side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentChange {
    pub uid: String,
    /// Persisted field changed → document dirty (`to_sim1` vs `saved_sim1`).
    pub saved: bool,
    /// Pin count / package / poles — drop wires to missing pins, full restamp.
    pub structural: bool,
    /// Live circuit must restamp or patch.
    pub sim: bool,
    /// Push undo (dialog, place, delete, gesture commit).
    pub undo: bool,
    pub view: ViewUpdate,
}

impl ComponentChange {
    /// Property dialog, switch toggle, place/delete, mouse on a persisted value.
    pub fn document(uid: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
            saved: true,
            structural: false,
            sim: true,
            undo: true,
            view: ViewUpdate::ReplaceItem,
        }
    }

    /// Momentary control while held (push button pressed). Not saved, no undo.
    pub fn live(uid: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
            saved: false,
            structural: false,
            sim: true,
            undo: false,
            view: ViewUpdate::Raster,
        }
    }

    /// High-frequency persisted value (knob, wiper, joystick). Raster + sim;
    /// no QML items rebuild and no undo snapshot per event.
    pub fn continuous(uid: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
            saved: true,
            structural: false,
            sim: true,
            undo: false,
            view: ViewUpdate::Raster,
        }
    }

    /// Pin-count / package / poles change.
    pub fn structural(uid: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
            saved: true,
            structural: true,
            sim: true,
            undo: true,
            view: ViewUpdate::ReplaceItem,
        }
    }

    pub fn no_undo(mut self) -> Self {
        self.undo = false;
        self
    }

    pub fn with_uid(mut self, uid: impl Into<String>) -> Self {
        self.uid = uid.into();
        self
    }

    pub fn with_fields(mut self, fields: &'static [&'static str]) -> Self {
        self.view = ViewUpdate::Fields(fields);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_sets_saved_undo_sim_view() {
        let c = ComponentChange::document("R-1");
        assert_eq!(c.uid, "R-1");
        assert!(c.saved && c.undo && c.sim);
        assert!(!c.structural);
        assert!(c.view.patches_item());
    }

    #[test]
    fn live_is_not_a_document_edit() {
        let c = ComponentChange::live("Push-1");
        assert!(!c.saved && !c.undo);
        assert!(c.sim);
        assert!(c.view.rasters_item());
        assert!(!c.view.rebuilds_qml_item());
    }

    #[test]
    fn continuous_rasters_without_qml_rebuild() {
        let c = ComponentChange::continuous("VoltSource-1");
        assert!(c.saved && c.sim && !c.undo);
        assert!(c.view.rasters_item());
        assert!(!c.view.rebuilds_qml_item());
    }

    #[test]
    fn fields_do_not_rebuild_qml_items() {
        let c = ComponentChange::document("Meter-1").with_fields(&["reading"]);
        assert!(c.view.rasters_item());
        assert!(!c.view.rebuilds_qml_item());
    }

    #[test]
    fn no_undo_keeps_saved() {
        let c = ComponentChange::document("Pot-1").no_undo();
        assert!(c.saved);
        assert!(!c.undo);
    }
}
