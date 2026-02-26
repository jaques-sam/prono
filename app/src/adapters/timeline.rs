use crate::Answer;
use egui::{Painter, Vec2};

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

pub fn extract_and_sort_dates(all_answers: Vec<(Option<&String>, Answer)>) -> Vec<(Option<&String>, TimelineDate)> {
    let mut dates: Vec<(Option<&String>, TimelineDate)> = all_answers
        .into_iter()
        .filter_map(|(user, answer)| match answer {
            Answer::PredictionDate { day: _, month, year } => Some((user, TimelineDate::new(year, month))),
            Answer::Text(_) => None,
        })
        .collect();

    dates.sort_by_key(|(_user, date)| date.months_since_epoch());
    dates
}

fn draw_month_ticks(ctx: &TimelineDrawContext, max_month: i32) {
    let min_year = (ctx.min_month / 12) + 2000;
    let max_year = (max_month / 12) + 2000;
    let color = ctx.painter.ctx().style().visuals.strong_text_color();

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

            if month == 1 {
                // Big tick for year
                ctx.painter.line_segment(
                    [
                        egui::pos2(x, ctx.rect.top() + ctx.line_y_offset - 8.0),
                        egui::pos2(x, ctx.rect.top() + ctx.line_y_offset + 8.0),
                    ],
                    egui::Stroke::new(1.0, color),
                );
                draw_label(ctx, &year.to_string(), x, None, false);
            } else {
                // Small tick for month
                ctx.painter.line_segment(
                    [
                        egui::pos2(x, ctx.rect.top() + ctx.line_y_offset - 4.0),
                        egui::pos2(x, ctx.rect.top() + ctx.line_y_offset + 4.0),
                    ],
                    egui::Stroke::new(0.2, color),
                );
            }
        }
    }
}

fn draw_label(ctx: &TimelineDrawContext, label: &str, x: f32, sub_label: Option<&String>, draw_above: bool) {
    let galley = ctx.painter.ctx().fonts_mut(|f| {
        f.layout_no_wrap(
            format!("{label}{}", sub_label.map(|s| format!(":\n{s}")).unwrap_or_default()),
            egui::FontId::new(10.0, egui::FontFamily::Proportional),
            egui::Color32::DARK_GRAY,
        )
    });
    let y_offset = ctx.rect.top() + ctx.line_y_offset + if draw_above { -40.0 } else { 15.0 };
    let x_offset = x + if draw_above { -galley.size().x / 2.0 } else { 0.0 };

    ctx.painter.add(egui::Shape::Text(egui::epaint::TextShape {
        pos: egui::pos2(x_offset, y_offset),
        galley,
        underline: egui::Stroke::NONE,
        fallback_color: egui::Color32::DARK_GREEN,
        override_text_color: None,
        opacity_factor: 0.8,
        angle: std::f32::consts::PI / 4.0, // 45°
    }));
}

fn draw_timeline_point(painter: &Painter, pos: egui::Pos2, hovered: bool) {
    let radius = if hovered { 5.0 } else { 3.0 };
    painter.circle_filled(pos, radius, egui::Color32::from_rgb(100, 255, 0));
    painter.circle_stroke(pos, radius, egui::Stroke::new(2.0, egui::Color32::BLACK));
}

fn draw_hover_tooltip(ctx: &TimelineDrawContext, label: &str, name: Option<&String>, x: f32, y: f32) {
    let color = ctx.painter.ctx().style().visuals.strong_text_color();

    // 45° diagonal going up from the bullet
    let diag_len = 15.0;
    let d = diag_len / std::f32::consts::SQRT_2;

    let start = egui::pos2(x, y - 5.0);
    let bend = egui::pos2(x + d, y - 5.0 - d);
    let horiz_end = egui::pos2(bend.x + 25.0, bend.y);

    let stroke = egui::Stroke::new(1.0, color);
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

    let bg_color = ctx.painter.ctx().style().visuals.window_fill;
    ctx.painter.rect_filled(text_rect, 2.0, bg_color);
    ctx.painter
        .rect_stroke(text_rect, 2.0, egui::Stroke::new(0.5, color), egui::StrokeKind::Middle);
    ctx.painter.galley(text_pos, galley, color);
}

pub fn draw(ui: &mut egui::Ui, dates: &[(Option<&String>, TimelineDate)]) {
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
    let min_month = *months.iter().min().unwrap_or(&0) - 1;
    let max_month = *months.iter().max().unwrap_or(&(min_month + 1)) + 1;
    let month_range = (max_month.checked_sub(min_month).unwrap_or(0)).max(1);

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
        egui::Stroke::new(3.0, egui::Color32::DARK_GRAY),
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

    // Find hovered point
    let hovered_idx = if response.hovered() {
        ui.ctx().input(|i| i.pointer.hover_pos()).and_then(|pointer_pos| {
            let hover_radius = 20.0_f32;
            let mut closest_idx = None;
            let mut closest_dist = f32::MAX;

            for (i, &(x, y)) in point_positions.iter().enumerate() {
                let dist = ((pointer_pos.x - x).powi(2) + (pointer_pos.y - y).powi(2)).sqrt();
                if dist < closest_dist && dist < hover_radius {
                    closest_dist = dist;
                    closest_idx = Some(i);
                }
            }
            closest_idx
        })
    } else {
        None
    };

    // Draw all points
    for (i, &(x, y)) in point_positions.iter().enumerate() {
        draw_timeline_point(&painter, egui::pos2(x, y), hovered_idx == Some(i));
    }

    // Draw tooltip for hovered point
    if let Some(idx) = hovered_idx {
        let (x, y) = point_positions[idx];
        draw_hover_tooltip(&ctx, &dates[idx].1.label, dates[idx].0, x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_date_creation() {
        let date = TimelineDate::new(2024, 5);
        assert_eq!(date.year, 2024);
        assert_eq!(date.month, 5);
        assert_eq!(date.label, "05/2024");
    }

    #[test]
    fn test_months_since_epoch() {
        let date_2000_01 = TimelineDate::new(2000, 1);
        assert_eq!(date_2000_01.months_since_epoch(), 1);

        let date_2000_12 = TimelineDate::new(2000, 12);
        assert_eq!(date_2000_12.months_since_epoch(), 12);

        let date_2001_01 = TimelineDate::new(2001, 1);
        assert_eq!(date_2001_01.months_since_epoch(), 13);

        let date_2024_05 = TimelineDate::new(2024, 5);
        assert_eq!(date_2024_05.months_since_epoch(), (24 * 12) + 5);
    }

    #[test]
    fn test_extract_dates_empty() {
        let dates = extract_and_sort_dates(Vec::new());
        assert!(dates.is_empty());
    }

    #[test]
    fn test_extract_and_sort_dates() {
        let user = Some(&String::from("user"));
        let all_answers = vec![
            (
                None,
                Answer::PredictionDate {
                    day: Some(15),
                    month: 5,
                    year: 2024,
                },
            ),
            (
                user,
                Answer::PredictionDate {
                    day: Some(20),
                    month: 3,
                    year: 2023,
                },
            ),
        ];

        let dates = extract_and_sort_dates(all_answers);
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0].0, Some(&String::from("user")));
        assert_eq!(dates[0].1.year, 2023);
        assert_eq!(dates[0].1.month, 3);
        assert_eq!(dates[1].0, None);
        assert_eq!(dates[1].1.year, 2024);
        assert_eq!(dates[1].1.month, 5);
    }

    #[test]
    fn test_extract_dates_does_not_remove_duplicates() {
        let user0 = Some(&String::from("Obiwan"));
        let user1 = Some(&String::from("Groku"));

        let all_answers = vec![
            (
                user0,
                Answer::PredictionDate {
                    day: Some(15),
                    month: 5,
                    year: 2024,
                },
            ),
            (
                user1,
                Answer::PredictionDate {
                    day: None,
                    month: 5,
                    year: 2024,
                },
            ), // Same year/month, should be deduplicated
        ];

        let dates = extract_and_sort_dates(all_answers);
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0].0, Some(&String::from("Obiwan")));
        assert_eq!(dates[0].1.year, 2024);
        assert_eq!(dates[0].1.month, 5);
        assert_eq!(dates[1].0, Some(&String::from("Groku")));
        assert_eq!(dates[1].1.year, 2024);
        assert_eq!(dates[1].1.month, 5);
    }

    #[test]
    fn test_extract_dates_ignores_text_answers() {
        let all_answers = vec![
            (
                None,
                Answer::PredictionDate {
                    day: Some(15),
                    month: 5,
                    year: 2024,
                },
            ),
            (None, Answer::Text(String::from("some text"))),
        ];

        let dates = extract_and_sort_dates(all_answers);
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0].1.year, 2024);
    }
}
