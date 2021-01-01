//! Selection state queries, hit testing predicates, and selection transformations.

use cs_engine::canvas::Canvas;
use cs_engine::components::ShapeKind;

pub(super) fn selected_item_uid(canvas: &Canvas) -> String {
    canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.selected)
        .map(|it| it.id.clone())
        .unwrap_or_default()
}

pub(super) fn selected_item_type(canvas: &Canvas) -> String {
    canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.selected)
        .map(|it| it.kind.type_name().to_string())
        .unwrap_or_default()
}

pub(super) fn selected_tunnel_visible(canvas: &Canvas) -> bool {
    canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.selected)
        .map(|it| it.tunnel_visible())
        .unwrap_or(true)
}

pub(super) fn selected_probe_pause(canvas: &Canvas) -> bool {
    canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.selected)
        .map(|it| it.probe_pause())
        .unwrap_or(false)
}

pub(super) fn selected_has_sd_image(canvas: &Canvas) -> bool {
    canvas
        .scene()
        .items()
        .iter()
        .find(|it| it.selected)
        .is_some_and(
            |it| matches!(&it.kind, cs_engine::canvas::Part::SdCard(sd) if !sd.file.is_empty()),
        )
}

pub(super) fn selected_has_image_data(canvas: &Canvas) -> bool {
    canvas.scene().items().iter().find(|it| it.selected).is_some_and(|it| {
        matches!(&it.kind, cs_engine::canvas::Part::Shape(shape) if shape.shape_kind == ShapeKind::Image && (shape.image_pixmap.is_some() || !shape.bck_data.is_empty() || !shape.image_file.is_empty()))
    })
}

pub(super) fn hit_pin_is_package(canvas: &Canvas) -> bool {
    let pin = canvas.hit_pin_id().unwrap_or("");
    if pin.is_empty() {
        return false;
    }
    let Some(item_id) = pin.rsplit_once('-').map(|(id, _)| id) else {
        return false;
    };
    canvas
        .scene()
        .item_by_id(item_id)
        .is_some_and(|it| matches!(&it.kind, cs_engine::canvas::Part::SubPackage(_)))
}
