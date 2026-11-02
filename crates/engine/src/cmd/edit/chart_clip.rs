//! Copying, cutting and pasting a chart. A pasted chart keeps its formatting and its data: in its
//! own workbook it refers to the same cells (series references always name their sheet, so it
//! shows the same data on any sheet); in another workbook it carries the values it showed when it
//! was copied, as Excel's charts do when their source book isn't open.

use gridcraft_chart::{ChartData, resolve};
use gridcraft_core::number_to_text;
use gridcraft_model::{Chart, ChartKind, ObjKind, ObjRef};
use serde_json::{Value as Json, json};

use super::{EngineError, Result, bad, cell_param_on, edit};
use crate::{ChartClip, Session};

/// Copies chart `id` on the active sheet to the clipboard (in place of any copied cells); a cut
/// also removes it, as one undo step.
pub(super) fn copy_chart(s: &mut Session, id: u32, cut: bool) -> Result<Json> {
    let d = s.doc()?;
    let sheet = d.wb.active_sheet();
    let sh = d.wb.sheet(sheet).ok_or(EngineError::NoDocument)?;
    let chart = sh.charts.iter().find(|c| c.id == id).cloned().ok_or_else(|| bad("chart", "no such chart on the active sheet"))?;
    let data = resolve(&d.wb, sheet, &chart);
    let clip = ChartClip { literal: with_values(&chart, &data), text: data_text(&chart, &data), chart, doc_uid: d.uid };
    let text = clip.text.clone();
    s.clipboard = None;
    s.chart_clip = Some(clip);
    if cut {
        edit(s, |cx| {
            cx.sheet_mut(sheet)?.remove_object(ObjRef::new(ObjKind::Chart, id));
            Ok(())
        })?;
    }
    Ok(json!({"chart": id, "text": text}))
}

/// Pastes the copied chart on the active sheet with its top-left corner at `at` (default: the
/// active cell), as a new chart of the same size.
pub(super) fn paste_chart(s: &mut Session, p: &Json) -> Result<Json> {
    let Some(clip) = s.chart_clip.clone() else { return Err(EngineError::Other("Nothing to paste.".into())) };
    let d = s.doc()?;
    let sheet = d.wb.active_sheet();
    let at = cell_param_on(s, p, "at", sheet)?.unwrap_or(d.selection.active);
    let mut chart = if clip.doc_uid == d.uid { clip.chart } else { clip.literal };
    let id = d.wb.next_object_id();
    chart.id = id;
    chart.anchor.cell = at;
    chart.anchor.dx = 0.0;
    chart.anchor.dy = 0.0;
    edit(s, |cx| {
        cx.sheet_mut(sheet)?.charts.push(chart);
        Ok(())
    })?;
    Ok(json!({"chart": id}))
}

/// Whether a paste of `text` (the system clipboard's, if the host passed it) pastes the copied
/// chart: the clipboard still holds what the chart copy put there.
pub(super) fn pastes_chart(s: &Session, text: Option<&str>) -> bool {
    let trim = |t: &str| t.trim_end_matches(['\r', '\n']).replace('\r', "");
    s.chart_clip.as_ref().is_some_and(|c| text.is_none_or(|t| trim(t) == trim(&c.text)))
}

/// `chart` with its references replaced by the values `data` resolved them to: series names,
/// categories (or X values), values and bubble sizes. Links that only make sense in the source
/// book (its PivotTable, its source block) are dropped.
fn with_values(chart: &Chart, data: &ChartData) -> Chart {
    let mut c = chart.clone();
    let numeric_x = matches!(c.kind, ChartKind::Scatter | ChartKind::ScatterLines | ChartKind::Bubble);
    for (i, se) in c.series.iter_mut().enumerate() {
        let Some(sd) = data.series.get(i) else { continue };
        se.name = Some(text_literal(&sd.name));
        se.values = number_array(&sd.values);
        se.categories = match (&sd.x, numeric_x) {
            (Some(x), true) => Some(number_array(x)),
            _ if se.categories.is_some() => Some(text_array(&data.categories)),
            _ => None,
        };
        se.bubble_sizes = se.bubble_sizes.as_ref().and(sd.sizes.as_deref()).map(number_array);
    }
    c.pivot = None;
    c.source = None;
    c
}

/// `{1,2,#N/A}`: a gap stays a gap.
fn number_array(v: &[Option<f64>]) -> String {
    let items: Vec<String> = v.iter().map(|n| n.map_or_else(|| "#N/A".to_string(), number_to_text)).collect();
    format!("{{{}}}", items.join(","))
}

/// `{"Jan","Feb"}`, quotes doubled.
fn text_array(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|t| text_literal(t)).collect();
    format!("{{{}}}", items.join(","))
}

fn text_literal(t: &str) -> String {
    format!("\"{}\"", t.replace('"', "\"\""))
}

/// The chart's data as tab-separated text, for the system clipboard: a header row of series
/// names, then one row per category. Never empty, so a paste can tell the copy is still there.
fn data_text(chart: &Chart, data: &ChartData) -> String {
    let rows = data.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    let mut out = String::new();
    let header: Vec<&str> = std::iter::once("").chain(data.series.iter().map(|s| s.name.as_str())).collect();
    out.push_str(&header.join("\t"));
    for r in 0..rows {
        out.push('\n');
        out.push_str(data.categories.get(r).map_or("", String::as_str));
        for s in &data.series {
            out.push('\t');
            out.push_str(&s.values.get(r).copied().flatten().map(number_to_text).unwrap_or_default());
        }
    }
    if data.series.is_empty() {
        out = chart.title.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| format!("Chart {}", chart.id));
    }
    out
}
