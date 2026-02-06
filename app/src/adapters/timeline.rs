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
    label_y_offset: f32,
    line_start_x: f32,
    usable_width: f32,
    month_range: i32,
    min_months: i32,
}

pub fn extract_dates(all_answers: Vec<Vec<&Answer>>) -> Vec<TimelineDate> {
    let mut dates: Vec<TimelineDate> = all_answers
        .into_iter()
        .flat_map(std::iter::IntoIterator::into_iter)
        .filter_map(|answer| match answer {
            Answer::PredictionDate { day: _, month, year } => Some(TimelineDate::new(*year, *month)),
            Answer::Text(_) => None,
        })
        .collect();

    // Sort by absolute month index then remove duplicates (same year+month)
    dates.sort_by_key(TimelineDate::months_since_epoch);
    dates.dedup_by(|a, b| a.year == b.year && a.month == b.month);
    dates
}

fn draw_month_ticks(ctx: &TimelineDrawContext, max_months: i32) {
    let min_year = (ctx.min_months / 12) + 2000;
    let max_year = (max_months / 12) + 2000;

    for year in min_year..=max_year {
        for month in 1..=12 {
            let months_since = (year - 2000) * 12 + month;
            if months_since < ctx.min_months || months_since > max_months {
                continue;
            }
            let months_offset = months_since - ctx.min_months;
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
                    egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                );
            } else {
                // Small tick for month
                ctx.painter.line_segment(
                    [
                        egui::pos2(x, ctx.rect.top() + ctx.line_y_offset - 4.0),
                        egui::pos2(x, ctx.rect.top() + ctx.line_y_offset + 4.0),
                    ],
                    egui::Stroke::new(0.2, egui::Color32::LIGHT_GRAY),
                );
            }
        }
    }
}

fn draw_timeline_point(painter: &Painter, pos: egui::Pos2, _date: &TimelineDate) {
    // Draw circle at the point
    painter.circle_filled(pos, 5.0, egui::Color32::from_rgb(100, 150, 255));
    // Draw border
    painter.circle_stroke(pos, 5.0, egui::Stroke::new(2.0, egui::Color32::DARK_BLUE));

    // Draw vertical line from the circle to the timeline
    painter.line_segment(
        [pos, egui::pos2(pos.x, pos.y + 5.0)],
        egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
    );
}

fn draw_labels(ctx: &TimelineDrawContext, date: &TimelineDate, x: f32) {
    // label rotated 90 degrees
    let label_pos = egui::pos2(x, ctx.rect.top() + ctx.label_y_offset);
    let galley = ctx.painter.ctx().fonts_mut(|f| {
        f.layout_no_wrap(
            date.label.clone(),
            egui::FontId::new(9.0, egui::FontFamily::Proportional),
            egui::Color32::DARK_GRAY,
        )
    });

    ctx.painter.add(egui::Shape::Text(egui::epaint::TextShape {
        pos: label_pos,
        galley,
        underline: egui::Stroke::NONE,
        fallback_color: egui::Color32::DARK_GREEN,
        override_text_color: None,
        opacity_factor: 0.8,
        angle: std::f32::consts::PI / 4.0, // 45°
    }));
}

pub fn draw(ui: &mut egui::Ui, dates: &[TimelineDate]) {
    if dates.is_empty() {
        ui.label("No dates entered yet");
        return;
    }

    let available_width = ui.available_width();
    let timeline_height = 80.0;
    let line_y_offset = 20.0; // vertical position of the timeline line within the rect
    let label_y_offset = line_y_offset + 15.0;

    // Create the painting area
    let (rect, _) = ui.allocate_exact_size(Vec2::new(available_width, timeline_height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    if dates.len() < 2 {
        // Center a single date
        let center_x = rect.left() + available_width / 2.0;
        draw_timeline_point(&painter, egui::pos2(center_x, rect.top() + line_y_offset), &dates[0]);
        return;
    }

    // Find min and max months to calculate scale
    let min_months = dates.iter().map(TimelineDate::months_since_epoch).min().unwrap_or(0);
    let max_months = dates
        .iter()
        .map(TimelineDate::months_since_epoch)
        .max()
        .unwrap_or(min_months + 1);
    let month_range = (max_months.checked_sub(min_months).unwrap_or(0)).max(1);

    let padding = 50.0;
    let usable_width = available_width - 2.0 * padding;
    let line_start_x = rect.left() + padding;
    let line_end_x = rect.right() - padding;

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
        label_y_offset,
        line_start_x,
        usable_width,
        month_range,
        min_months,
    };

    draw_month_ticks(&ctx, max_months);

    // Draw points and labels for each date
    for date in dates {
        let months_offset = date.months_since_epoch() - min_months;
        #[allow(clippy::cast_precision_loss)] // month range is 23 bit, which is still millions of years
        let progress = months_offset as f32 / month_range as f32;
        let x = line_start_x + progress * usable_width;

        draw_timeline_point(&painter, egui::pos2(x, rect.top() + line_y_offset), date);
        draw_labels(&ctx, date, x);
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
        let dates = extract_dates(Vec::new());
        assert!(dates.is_empty());
    }

    #[test]
    fn test_extract_dates_with_predictions() {
        let all_answers = vec![vec![
            &Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2024,
            },
            &Answer::PredictionDate {
                day: Some(20),
                month: 3,
                year: 2023,
            },
        ]];

        let dates = extract_dates(all_answers);
        assert_eq!(dates.len(), 2);
        // Should be sorted by month since epoch
        assert_eq!(dates[0].year, 2023);
        assert_eq!(dates[0].month, 3);
        assert_eq!(dates[1].year, 2024);
        assert_eq!(dates[1].month, 5);
    }

    #[test]
    fn test_extract_dates_with_optional_day() {
        let all_answers = vec![vec![
            &Answer::PredictionDate {
                day: None,
                month: 5,
                year: 2024,
            },
            &Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2024,
            },
        ]];

        let dates = extract_dates(all_answers);
        assert_eq!(dates.len(), 1);
        // Both should be treated the same since only year/month matters
        assert_eq!(dates[0].year, 2024);
        assert_eq!(dates[0].month, 5);
    }

    #[test]
    fn test_extract_dates_deduplication() {
        let all_answers = vec![vec![
            &Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2024,
            },
            &Answer::PredictionDate {
                day: Some(20),
                month: 5,
                year: 2024,
            }, // Same year/month, should be deduplicated
        ]];

        let dates = extract_dates(all_answers);
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0].year, 2024);
        assert_eq!(dates[0].month, 5);
    }

    #[test]
    fn test_extract_dates_ignores_text_answers() {
        let answer = Answer::Text(String::from("some text"));
        let all_answers = vec![vec![
            &Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2024,
            },
            &answer,
        ]];

        let dates = extract_dates(all_answers);
        assert_eq!(dates.len(), 1);
        assert_eq!(dates[0].year, 2024);
    }
}
