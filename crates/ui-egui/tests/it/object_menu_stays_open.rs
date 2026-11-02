//! A right-click on a chart opens the chart's menu, and the menu stays open: a trackpad's
//! two-finger click presses and releases in one frame, and that press opened the menu, so it
//! mustn't count as a click elsewhere and close it again.

use egui::{Event, Modifiers, PointerButton, Pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_engine::core::CellRef;
use gridcraft_ui_egui::{SheetApp, grid::Geo};
use serde_json::json;

type Harness = egui_kittest::Harness<'static, SheetApp>;

/// `ppp` physical pixels per point: 2 on a Retina display.
fn harness_at(ppp: f32) -> Harness {
    let mut session = Session::new();
    session.new_workbook();
    session.execute("range.setValues", json!({"range": "A1", "values": [[1], [2], [3]]})).unwrap();
    session.execute("selection.set", json!({"range": "A1:A3"})).unwrap();
    session.execute("insert.chart", json!({"type": "column", "at": "D3", "width": 300, "height": 200})).unwrap();
    let mut h = crate::common::builder().with_size(vec2(1200.0, 800.0)).with_pixels_per_point(ppp).build_ui_state(
        |ui, app: &mut SheetApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        crate::common::app(session),
    );
    h.run_steps(4);
    h
}

/// A point inside the chart, away from its edges and handles.
fn inside_chart(h: &Harness) -> Pos2 {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    let geo = Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
    geo.cell_rect(sh, CellRef::parse("D3").unwrap()).min + vec2(150.0, 100.0) * geo.z
}

fn button(h: &mut Harness, pos: Pos2, pressed: bool) {
    h.input_mut().events.push(Event::PointerButton { pos, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
}

fn quick_right_click(h: &mut Harness, pos: Pos2) {
    h.input_mut().events.push(Event::PointerMoved(pos));
    button(h, pos, true);
    button(h, pos, false);
    h.run_steps(4);
}

#[test]
fn a_quick_right_click_keeps_the_chart_menu_open() {
    for ppp in [1.0, 2.0] {
        let mut h = harness_at(ppp);
        // A real pointer sits between pixels; the menu's corner snaps to the nearest one.
        let pos = inside_chart(&h) + vec2(0.46, 0.37);
        quick_right_click(&mut h, pos);
        assert!(h.state().grid.object_menu.is_some(), "the chart menu stays open (ppp {ppp})");
    }
}

#[test]
fn a_slow_right_click_keeps_the_chart_menu_open() {
    let mut h = harness_at(1.0);
    let pos = inside_chart(&h);
    h.input_mut().events.push(Event::PointerMoved(pos));
    h.run_steps(1);
    button(&mut h, pos, true);
    h.run_steps(1);
    button(&mut h, pos, false);
    h.run_steps(4);
    assert!(h.state().grid.object_menu.is_some(), "the chart menu stays open");
}

#[test]
fn a_click_elsewhere_closes_the_chart_menu() {
    let mut h = harness_at(1.0);
    let pos = inside_chart(&h);
    quick_right_click(&mut h, pos);
    assert!(h.state().grid.object_menu.is_some());
    let away = pos2_away(&h);
    h.input_mut().events.push(Event::PointerMoved(away));
    h.input_mut().events.push(Event::PointerButton { pos: away, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.input_mut().events.push(Event::PointerButton { pos: away, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(4);
    assert!(h.state().grid.object_menu.is_none(), "a click elsewhere closes it");
}

/// A cell far from the chart and its menu.
fn pos2_away(h: &Harness) -> Pos2 {
    let app = h.state();
    let sh = app.session.doc().unwrap().wb.active().unwrap();
    let geo = Geo::new(sh, app.grid.rect.unwrap(), app.view().scroll);
    geo.cell_rect(sh, CellRef::parse("M20").unwrap()).center()
}
