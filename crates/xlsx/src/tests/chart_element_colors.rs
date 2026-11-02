//! Chart element colours (background, border, plot area, text and lines) through save and reopen,
//! for DrawingML and chartex charts.

use gridcraft_model::{Anchor, Chart, ChartColors, ChartKind, Color, LegendPos, Series, Workbook};

use crate::{chart::read_chart, read_xlsx, write_xlsx, xml::parse};

fn all_colors() -> ChartColors {
    let c = |n: u32| Some(Color::Rgb(0x102000 + n));
    ChartColors {
        background: c(1),
        border: c(2),
        plot_area: c(3),
        title: c(4),
        axis_titles: c(5),
        labels: c(6),
        legend: c(7),
        axis_lines: c(8),
        gridlines: c(9),
    }
}

fn chart(kind: ChartKind, colors: ChartColors) -> Chart {
    Chart {
        id: 1,
        kind,
        anchor: Anchor { width: 480.0, height: 300.0, ..Anchor::default() },
        title: Some("Sales".into()),
        series: vec![Series {
            name: Some("Sheet1!$B$1".into()),
            categories: Some("Sheet1!$A$2:$A$4".into()),
            values: "Sheet1!$B$2:$B$4".into(),
            ..Default::default()
        }],
        legend: LegendPos::Right,
        data_labels: true,
        data_table: None,
        gridlines: true,
        style: 2,
        x_title: Some("Month".into()),
        y_title: Some("Total".into()),
        source: None,
        by_rows: false,
        pivot: None,
        colors,
        font: None,
        font_size: None,
    }
}

fn reopen(c: Chart) -> Chart {
    let mut wb = Workbook::new();
    wb.sheet_mut(0).unwrap().charts.push(c);
    let (back, _) = read_xlsx(&write_xlsx(&wb).unwrap()).unwrap();
    back.sheets[0].charts[0].clone()
}

#[test]
fn every_element_color_survives_save_and_reopen() {
    // Charts with both axes keep every colour; a pie has no axes, so keeps the others.
    let no_axes = ChartColors { axis_titles: None, axis_lines: None, gridlines: None, plot_area: None, ..all_colors() };
    for (kind, expected) in [
        (ChartKind::ColumnClustered, all_colors()),
        (ChartKind::BarClustered, all_colors()),
        (ChartKind::Line, all_colors()),
        (ChartKind::Scatter, all_colors()),
        (ChartKind::Waterfall, all_colors()),
        (ChartKind::Histogram, all_colors()),
        (ChartKind::Pie, no_axes),
    ] {
        let mut sent = all_colors();
        if kind == ChartKind::Pie {
            sent.plot_area = None;
        }
        let back = reopen(chart(kind, sent));
        assert_eq!(back.kind, kind);
        assert_eq!(back.colors, expected, "{kind:?}");
        // And again, so a reopened file saves the same colours.
        assert_eq!(reopen(back).colors, expected, "{kind:?} second save");
    }
}

#[test]
fn some_picked_colors_leave_the_rest_built_in() {
    for kind in [ChartKind::ColumnClustered, ChartKind::Waterfall] {
        let colors = ChartColors { background: Some(Color::Rgb(0x1E1E2E)), gridlines: Some(Color::Rgb(0x585B70)), ..Default::default() };
        assert_eq!(reopen(chart(kind, colors.clone())).colors, colors, "{kind:?}");
        assert_eq!(reopen(chart(kind, ChartColors::default())).colors, ChartColors::default(), "{kind:?}");
    }
}

#[test]
fn unpicked_charts_write_no_element_styling() {
    let xml = crate::chart::write_chart(&Workbook::new(), 0, &chart(ChartKind::ColumnClustered, ChartColors::default()), "Book.xlsx");
    assert!(!xml.contains("txPr") && !xml.contains("spPr") && !xml.contains("rPr"), "{xml}");
}

#[test]
fn excel_theme_styling_is_not_read_as_picked_colors() {
    // Excel's defaults use scheme colours, often tinted: they keep the built-in look.
    let xml = "<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><c:chart>\
        <c:plotArea><c:barChart><c:barDir val=\"col\"/><c:ser><c:idx val=\"0\"/><c:val><c:numLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>\
        <c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx><c:axId val=\"1\"/><c:spPr><a:ln><a:solidFill><a:schemeClr val=\"tx1\"><a:lumMod val=\"15000\"/><a:lumOff val=\"85000\"/></a:schemeClr></a:solidFill></a:ln></c:spPr></c:catAx>\
        <c:valAx><c:axId val=\"2\"/><c:majorGridlines><c:spPr><a:ln><a:solidFill><a:srgbClr val=\"D9D9D9\"><a:alpha val=\"50000\"/></a:srgbClr></a:solidFill></a:ln></c:spPr></c:majorGridlines></c:valAx>\
        <c:spPr><a:noFill/></c:spPr></c:plotArea><c:legend><c:legendPos val=\"r\"/><c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr><a:solidFill><a:schemeClr val=\"tx1\"/></a:solidFill></a:defRPr></a:pPr></a:p></c:txPr></c:legend></c:chart>\
        <c:spPr><a:solidFill><a:schemeClr val=\"bg1\"/></a:solidFill></c:spPr></c:chartSpace>";
    assert_eq!(read_chart(&parse(xml.as_bytes()).unwrap()).unwrap().colors, ChartColors::default());
}

#[test]
fn a_chart_font_survives_save_and_reopen() {
    for kind in [ChartKind::ColumnClustered, ChartKind::Pie, ChartKind::Waterfall] {
        let mut c = chart(kind, ChartColors { title: Some(Color::Rgb(0x123456)), ..Default::default() });
        c.font = Some("IBM Plex Mono".into());
        let back = reopen(c);
        assert_eq!(back.font.as_deref(), Some("IBM Plex Mono"), "{kind:?}");
        assert_eq!(back.colors.title, Some(Color::Rgb(0x123456)), "{kind:?}: the font leaves colours alone");
        assert_eq!(reopen(chart(kind, ChartColors::default())).font, None, "{kind:?}");
    }
    // A size, with or without a typeface.
    for (font, size) in [(Some("IBM Plex Mono"), Some(12.5)), (None, Some(16.0)), (None, None)] {
        for kind in [ChartKind::ColumnClustered, ChartKind::Waterfall] {
            let mut c = chart(kind, ChartColors::default());
            c.font = font.map(String::from);
            c.font_size = size;
            let back = reopen(c);
            assert_eq!((back.font.as_deref(), back.font_size), (font, size), "{kind:?}");
        }
    }
    // A name needing escapes comes back as written.
    let mut c = chart(ChartKind::Line, ChartColors::default());
    c.font = Some("Tom & \"Jerry\" <Sans>".into());
    assert_eq!(reopen(c).font.as_deref(), Some("Tom & \"Jerry\" <Sans>"));
}

#[test]
fn a_theme_font_is_not_read_as_a_chart_font() {
    for face in ["+mn-lt", "+mj-lt", ""] {
        let xml = format!(
            "<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><c:chart>\
            <c:plotArea><c:barChart><c:barDir val=\"col\"/><c:ser><c:idx val=\"0\"/><c:val><c:numLit><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>\
            <c:axId val=\"1\"/><c:axId val=\"2\"/></c:barChart><c:catAx/><c:valAx/></c:plotArea></c:chart>\
            <c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr><a:latin typeface=\"{face}\"/></a:defRPr></a:pPr></a:p></c:txPr></c:chartSpace>"
        );
        assert_eq!(read_chart(&parse(xml.as_bytes()).unwrap()).unwrap().font, None, "{face:?}");
    }
}

#[test]
fn point_colours_survive_save_and_reopen() {
    for kind in [
        ChartKind::ColumnClustered,
        ChartKind::BarClustered,
        ChartKind::Line,
        ChartKind::LineMarkers,
        ChartKind::Area,
        ChartKind::Pie,
        ChartKind::Doughnut,
        ChartKind::Scatter,
        ChartKind::Bubble,
        ChartKind::Radar,
        ChartKind::Waterfall,
        ChartKind::Funnel,
        ChartKind::Treemap,
    ] {
        let mut c = chart(kind, ChartColors::default());
        c.series[0].set_point_color(1, Some(Color::Rgb(0xC0392B)));
        c.series[0].set_point_color(0, Some(Color::Rgb(0x2E86C1)));
        let back = reopen(c.clone());
        assert_eq!(back.series[0].point_colors, c.series[0].point_colors, "{kind:?}");
        assert_eq!(reopen(back).series[0].point_colors, c.series[0].point_colors, "{kind:?} second save");
    }
    // Without picks, no point colours appear (Excel's own varied palette doesn't count).
    for kind in [ChartKind::Pie, ChartKind::Treemap, ChartKind::Waterfall] {
        assert!(reopen(chart(kind, ChartColors::default())).series[0].point_colors.is_empty(), "{kind:?}");
    }
}
