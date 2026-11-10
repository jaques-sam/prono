use egui::{Color32, Painter, Vec2};
use prono_api::Answer;

/// The fill of a point: one per predicted date.
pub const POINT: Color32 = Color32::from_rgb(100, 255, 0);
/// The fill of the viewer's own points, so they stand out among the others.
pub const OWN_POINT: Color32 = Color32::from_rgb(255, 140, 0);
/// What the callout of the viewer's own point says instead of their name.
pub const OWN_LABEL: &str = "You";

#[derive(Clone, Debug)]
pub struct TimelineDate {
    pub year: u16,
    pub month: u8,
    pub label: String,
}

impl TimelineDate {
    pub fn new(year: u16, month: u8) -> Self {
        Self {
            year,
            month,
            label: format!("{month:02}/{year}"),
        }
    }

    pub fn months_since_epoch(&self) -> i32 {
        (i32::from(self.year) - 2000) * 12 + i32::from(self.month)
    }
}

struct TimelineDrawContext {
    rect: egui::Rect,
    painter: Painter,
    line_y_offset: f32,
    line_start_x: f32,
    usable_width: f32,
    month_range: i32,
    min_month: i32,
}

/// The predicted dates among `answers`, oldest first, each with the name of
/// who predicted it (if shown).
pub fn extract_and_sort_dates<'a>(
    answers: impl IntoIterator<Item = (Option<&'a str>, &'a Answer)>,
) -> Vec<(Option<&'a str>, TimelineDate)> {
    let mut dates: Vec<_> = answers
        .into_iter()
        .filter_map(|(user, answer)| match answer {
            Answer::PredictionDate { month, year, .. } => Some((user, TimelineDate::new(*year, *month))),
            Answer::Text(_) | Answer::Poll(_) => None,
        })
        .collect();

    dates.sort_by_key(|(_user, date)| date.months_since_epoch());
    dates
}

fn draw_month_ticks(ctx: &TimelineDrawContext, max_month: i32) {
    let min_year = (ctx.min_month / 12) + 2000;
    let max_year = (max_month / 12) + 2000;
    let color = ctx.painter.ctx().global_style().visuals.strong_text_color();

    for year in min_year..=max_year {
        for month in 1..=12 {
            let months_since = (year - 2000) * 12 + month;
            if months_since < ctx.min_month || months_since > max_month {
                continue;
            }
            let months_offset = months_since - ctx.min_month;
            #[allow(clippy::cast_precision_loss)]
            let progress = months_offset as f32 / ctx.month_range as f32;
            let x = ctx.line_start_x + progress * ctx.usable_width;

            let (half_height, width): (f32, f32) = if month == 1 { (8.0, 1.0) } else { (4.0, 0.2) };
            ctx.painter.line_segment(
                [
                    egui::pos2(x, ctx.rect.top() + ctx.line_y_offset - half_height),
                    egui::pos2(x, ctx.rect.top() + ctx.line_y_offset + half_height),
                ],
                egui::Stroke::new(width, color),
            );
            if month == 1 {
                draw_year_label(ctx, &year.to_string(), x);
            }
        }
    }
}

fn draw_year_label(ctx: &TimelineDrawContext, label: &str, x: f32) {
    let galley = ctx.painter.ctx().fonts_mut(|f| {
        f.layout_no_wrap(
            label.to_string(),
            egui::FontId::new(10.0, egui::FontFamily::Proportional),
            egui::Color32::DARK_GRAY,
        )
    });

    ctx.painter.add(egui::Shape::Text(egui::epaint::TextShape {
        pos: egui::pos2(x, ctx.rect.top() + ctx.line_y_offset + 15.0),
        galley,
        underline: egui::Stroke::NONE,
        fallback_color: egui::Color32::DARK_GREEN,
        override_text_color: None,
        opacity_factor: 0.8,
        angle: std::f32::consts::PI / 4.0, // 45°
    }));
}

fn draw_timeline_point(painter: &Painter, pos: egui::Pos2, hovered: bool, own: bool) {
    let radius = match (own, hovered) {
        (false, false) => 3.0,
        (false, true) | (true, false) => 5.0,
        (true, true) => 7.0,
    };
    painter.circle_filled(pos, radius, if own { OWN_POINT } else { POINT });
    painter.circle_stroke(pos, radius, egui::Stroke::new(2.0_f32, egui::Color32::BLACK));
}

/// A callout above the point at `x`, `y`: the date, and who predicted it if
/// known.
fn draw_callout(ctx: &TimelineDrawContext, label: &str, name: Option<&str>, x: f32, y: f32) {
    let color = ctx.painter.ctx().global_style().visuals.strong_text_color();

    // 45° diagonal going up from the bullet
    let diag_len = 15.0;
    let d = diag_len / std::f32::consts::SQRT_2;

    let start = egui::pos2(x, y - 5.0);
    let bend = egui::pos2(x + d, y - 5.0 - d);
    let horiz_end = egui::pos2(bend.x + 25.0, bend.y);

    let stroke = egui::Stroke::new(1.0_f32, color);
    ctx.painter.line_segment([start, bend], stroke);
    ctx.painter.line_segment([bend, horiz_end], stroke);

    let text = match name {
        Some(n) => format!("{label}:\n{n}"),
        None => label.to_string(),
    };
    let galley = ctx
        .painter
        .ctx()
        .fonts_mut(|f| f.layout_no_wrap(text, egui::FontId::new(11.0, egui::FontFamily::Proportional), color));

    let text_pos = egui::pos2(horiz_end.x + 3.0, horiz_end.y - galley.size().y / 2.0);
    let text_rect = egui::Rect::from_min_size(text_pos, galley.size()).expand(2.0);

    let bg_color = ctx.painter.ctx().global_style().visuals.window_fill;
    ctx.painter.rect_filled(text_rect, 2.0, bg_color);
    ctx.painter.rect_stroke(
        text_rect,
        2.0,
        egui::Stroke::new(0.5_f32, color),
        egui::StrokeKind::Middle,
    );
    ctx.painter.galley(text_pos, galley, color);
}

/// Draws the timeline of `dates`. The points of `own_name` stand out and are
/// always labelled, so the viewer finds their own answer among the others.
pub fn draw(ui: &mut egui::Ui, dates: &[(Option<&str>, TimelineDate)], own_name: Option<&str>) {
    if dates.is_empty() {
        ui.label("No dates entered yet");
        return;
    }

    let available_width = ui.available_width();
    let timeline_height = 100.0;
    let line_y_offset = 50.0; // vertical position of the timeline line within the rect

    // Create the painting area
    let (rect, response) = ui.allocate_exact_size(Vec2::new(available_width, timeline_height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let months: Vec<i32> = dates.iter().map(|(_, d)| d.months_since_epoch()).collect();
    let min_month = months.iter().min().copied().unwrap_or(0) - 1;
    let max_month = months.iter().max().copied().unwrap_or(min_month) + 1;
    let month_range = (max_month - min_month).max(1);

    let padding_left = 25.0;
    let padding_right = 100.0;
    let usable_width = available_width - padding_left - padding_right;
    let line_start_x = rect.left() + padding_left;
    let line_end_x = rect.right() - padding_right;

    // Draw the horizontal timeline line
    painter.line_segment(
        [
            egui::pos2(line_start_x, rect.top() + line_y_offset),
            egui::pos2(line_end_x, rect.top() + line_y_offset),
        ],
        egui::Stroke::new(3.0_f32, egui::Color32::DARK_GRAY),
    );

    let ctx = TimelineDrawContext {
        rect,
        painter: painter.clone(),
        line_y_offset,
        line_start_x,
        usable_width,
        month_range,
        min_month,
    };

    // Compute positions for all points, spreading overlapping ones vertically
    let y_spread = 6.0;
    let point_positions: Vec<(f32, f32)> = dates
        .iter()
        .enumerate()
        .map(|(i, date)| {
            let month_val = date.1.months_since_epoch();
            #[allow(clippy::cast_precision_loss)]
            let x = if dates.len() < 2 {
                rect.left() + available_width / 2.0
            } else {
                let progress = (month_val - min_month) as f32 / month_range as f32;
                line_start_x + progress * usable_width
            };

            // Spread overlapping points vertically
            let same_before = dates[..i]
                .iter()
                .filter(|d| d.1.months_since_epoch() == month_val)
                .count();
            let same_total = dates.iter().filter(|d| d.1.months_since_epoch() == month_val).count();

            #[allow(clippy::cast_precision_loss)]
            let y = if same_total > 1 {
                let offset = same_before as f32 - (same_total - 1) as f32 / 2.0;
                rect.top() + line_y_offset + offset * y_spread
            } else {
                rect.top() + line_y_offset
            };

            (x, y)
        })
        .collect();

    if dates.len() >= 2 {
        draw_month_ticks(&ctx, max_month);
    }

    // Find the hovered point
    let hovered_idx = response
        .hover_pos()
        .and_then(|pointer| {
            point_positions
                .iter()
                .enumerate()
                .map(|(i, &(x, y))| (i, pointer.distance(egui::pos2(x, y))))
                .filter(|&(_, dist)| dist < 20.0)
                .min_by(|a, b| a.1.total_cmp(&b.1))
        })
        .map(|(i, _)| i);

    let is_own = |i: usize| own_name.is_some() && dates[i].0 == own_name;
    // Own points last, so no other point hides them.
    let draw_order = (0..dates.len())
        .filter(|&i| !is_own(i))
        .chain((0..dates.len()).filter(|&i| is_own(i)));
    for i in draw_order {
        let (x, y) = point_positions[i];
        draw_timeline_point(&painter, egui::pos2(x, y), hovered_idx == Some(i), is_own(i));
    }

    // Callouts: the hovered point's, over the always shown ones of own points.
    let callout = |i: usize| {
        let (x, y) = point_positions[i];
        let name = if is_own(i) { Some(OWN_LABEL) } else { dates[i].0 };
        draw_callout(&ctx, &dates[i].1.label, name, x, y);
    };
    (0..dates.len())
        .filter(|&i| is_own(i) && hovered_idx != Some(i))
        .for_each(callout);
    if let Some(i) = hovered_idx {
        callout(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u8, year: u16) -> Answer {
        Answer::PredictionDate { day: None, month, year }
    }

    #[test]
    fn timeline_date_label() {
        assert_eq!(TimelineDate::new(2024, 5).label, "05/2024");
    }

    #[test]
    fn months_since_epoch() {
        assert_eq!(TimelineDate::new(2000, 1).months_since_epoch(), 1);
        assert_eq!(TimelineDate::new(2000, 12).months_since_epoch(), 12);
        assert_eq!(TimelineDate::new(2001, 1).months_since_epoch(), 13);
        assert_eq!(TimelineDate::new(2024, 5).months_since_epoch(), 24 * 12 + 5);
    }

    #[test]
    fn dates_are_sorted_keeping_duplicates_and_dropping_text() {
        let answers = [date(5, 2024), date(3, 2023), date(5, 2024), Answer::Text("soon".into())];
        let users = [Some("a"), Some("b"), Some("c"), Some("d")];

        let dates = extract_and_sort_dates(users.into_iter().zip(&answers));

        let order: Vec<_> = dates.iter().map(|(user, d)| (*user, d.year, d.month)).collect();
        assert_eq!(
            order,
            [(Some("b"), 2023, 3), (Some("a"), 2024, 5), (Some("c"), 2024, 5)]
        );
    }

    #[test]
    fn no_answers_no_dates() {
        assert!(extract_and_sort_dates(std::iter::empty()).is_empty());
    }
}
