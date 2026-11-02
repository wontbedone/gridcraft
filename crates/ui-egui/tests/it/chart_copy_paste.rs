//! Copy a chart with Cmd+C, paste it on another sheet with Cmd+V: the copy is a new chart with the
//! same data and design, and it is selected, as an inserted chart is.

use egui::{Event, Modifiers, PointerButton, Pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_engine::model::{ObjKind, ObjRef};
use gridcraft_ui_egui::{SheetApp, grid::Geo};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> (Harness, u32) {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [["", "Sales"], ["Jan", 3], ["Feb", 5]]})).unwrap();
    session.execute("selection.set", json!({"range": "A1:B3"})).unwrap();
    let id = session.execute("insert.chart", json!({"type": "column", "at": "D3", "width": 300, "height": 200})).unwrap()["chart"].as_u64().unwrap()
        as u32;
    session.execute("chart.setColors", json!({"background": "#1E1E2E"})).unwrap();
    session.execute("home.insertSheet", json!({"name": "Two"})).unwrap();
    session.execute("sheet.activate", json!({"sheet": 0})).unwrap();
    let mut h = crate::common::builder().with_size(vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    (h, id)
}

fn inside_chart(h: &Harness) -> Pos2 {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    let geo = Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
    geo.cell_rect(sh, CellRef::parse("D3").unwrap()).min + vec2(150.0, 100.0) * geo.z
}

#[test]
fn a_chart_copied_with_cmd_c_pastes_on_another_sheet_with_cmd_v() {
    let (mut h, id) = harness();
    // Click the chart to select it.
    let pos = inside_chart(&h);
    h.input_mut().events.push(Event::PointerMoved(pos));
    h.input_mut().events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.input_mut().events.push(Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    assert_eq!(h.state().selected_object, Some(ObjRef::new(ObjKind::Chart, id)));
    h.input_mut().events.push(Event::Copy);
    h.run_steps(3);
    let text = h.state().session.chart_clip.as_ref().map(|c| c.text.clone()).expect("the chart was copied");
    // Another sheet, then Cmd+V: the host hands over the clipboard text the copy put there.
    h.state_mut().run_or_alert("sheet.activate", json!({"sheet": 1}));
    h.run_steps(3);
    h.input_mut().events.push(Event::Paste(text));
    h.run_steps(3);
    let app = h.state();
    let wb = &app.session.doc().unwrap().wb;
    let (source, copy) = (&wb.sheet(0).unwrap().charts[0], wb.sheet(1).unwrap().charts.first().expect("pasted on Two"));
    assert_eq!((&copy.series, &copy.colors, copy.kind), (&source.series, &source.colors, source.kind));
    assert_eq!(app.selected_object, Some(ObjRef::new(ObjKind::Chart, copy.id)), "the pasted chart is selected");
    assert_eq!(wb.sheet(0).unwrap().charts.len(), 1, "the source stays");
}
