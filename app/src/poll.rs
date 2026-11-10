//! The poll question: radio buttons to answer it and a bar chart of what
//! everyone answered. The counting lives in `Question::poll_tally`; this
//! module turns it into rows and draws them, like [`crate::timeline`].

use std::collections::HashSet;

use egui::{CornerRadius, Id, RichText, Sense, Stroke, StrokeKind, TextStyle, TextWrapMode, Vec2};
use prono_api::{Question, UserAnswer};

/// The text of the radio button for an answer outside the given options.
const OTHER_LABEL: &str = "Other";

/// The widest part of the row the labels may take.
const MAX_LABEL_SHARE: f32 = 0.4;
/// Space between the label, the bar and the count.
const GAP: f32 = 8.0;
/// The narrowest bar of an option that got votes, so it stays visible.
const MIN_BAR_WIDTH: f32 = 4.0;
/// Vertical padding of a row, around its text.
const ROW_PADDING: f32 = 4.0;
const BAR_RADIUS: u8 = 4;

/// One row of the bar chart: an option, or an "other" answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The option or the typed answer.
    pub text: String,
    pub votes: usize,
    /// Whether this is a typed answer, not one of the question's options.
    pub other: bool,
}

impl Row {
    /// What the row is called in the chart.
    pub fn label(&self) -> String {
        if self.other {
            format!("{} (other)", self.text)
        } else {
            self.text.clone()
        }
    }

    /// `3 votes (60%)` of `total` votes.
    pub fn summary(&self, total: usize) -> String {
        let noun = if self.votes == 1 { "vote" } else { "votes" };
        format!("{} {noun} ({:.0}%)", self.votes, percentage(self.votes, total))
    }
}

/// The rows of the chart of `question`: its options in order, then the other
/// answers.
pub fn rows(question: &Question, answers: &[UserAnswer]) -> Vec<Row> {
    question
        .poll_tally(answers)
        .into_iter()
        .enumerate()
        .map(|(index, (text, votes))| Row {
            text,
            votes,
            other: index >= question.possible_answers.len(),
        })
        .collect()
}

/// The share of `votes` among `total`, in percent.
#[allow(clippy::cast_precision_loss)]
pub fn percentage(votes: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        votes as f32 * 100.0 / total as f32
    }
}

/// The width of the bar for `votes`, where `max_votes` fills `max_width`.
#[allow(clippy::cast_precision_loss)]
pub fn bar_width(votes: usize, max_votes: usize, max_width: f32) -> f32 {
    if votes == 0 || max_votes == 0 {
        return 0.0;
    }
    (votes as f32 / max_votes as f32 * max_width).max(MIN_BAR_WIDTH.min(max_width))
}

/// Whether the "Other" radio button is selected: the participant chose it, or
/// the answer is text that is none of the `options` (an answer filled in
/// again).
pub fn other_selected(options: &[String], answer: &str, chosen: bool) -> bool {
    chosen || (!answer.is_empty() && !options.contains(&answer.to_string()))
}

/// The questions whose "Other" radio button the participant selected. An
/// empty answer is both "nothing chosen" and "Other, not typed yet", so this
/// one bit lives here, in egui's memory, not in the answers.
#[derive(Clone, Default)]
struct OtherChosen(HashSet<Id>);

fn other_chosen_id() -> Id {
    Id::new("poll_other_chosen")
}

/// Forgets every selection of "Other": the answers were reset or left.
pub fn forget_other(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<OtherChosen>(other_chosen_id()));
}

/// Draws the radio buttons of `question` and writes the choice to `answer`:
/// the option, or whatever is typed next to "Other".
pub fn input(ui: &mut egui::Ui, question: &Question, answer: &mut String) {
    let key = Id::new(question.id);
    let chosen = ui.data(|data| {
        data.get_temp::<OtherChosen>(other_chosen_id())
            .is_some_and(|chosen| chosen.0.contains(&key))
    });
    let mut other = other_selected(&question.possible_answers, answer, chosen);

    ui.vertical(|ui| {
        for option in &question.possible_answers {
            if ui.radio(!other && answer == option, option).clicked() {
                answer.clone_from(option);
                other = false;
            }
        }
        if ui.radio(other, OTHER_LABEL).clicked() && !other {
            answer.clear();
            other = true;
        }
        if other {
            ui.add(egui::TextEdit::singleline(answer).hint_text("your own answer"));
        }
    });

    ui.data_mut(|data| {
        let chosen = data.get_temp_mut_or_default::<OtherChosen>(other_chosen_id());
        if other {
            chosen.0.insert(key);
        } else {
            chosen.0.remove(&key);
        }
    });
}

/// Draws `rows` as horizontal bars: the label, a bar as long as the votes,
/// and the votes.
pub fn draw(ui: &mut egui::Ui, rows: &[Row]) {
    let total: usize = rows.iter().map(|row| row.votes).sum();
    let max_votes = rows.iter().map(|row| row.votes).max().unwrap_or(0);
    let width = ui.available_width();

    let count_galleys: Vec<_> = rows
        .iter()
        .map(|row| plain_galley(ui, &row.votes.to_string(), f32::INFINITY))
        .collect();
    let count_width = count_galleys.iter().map(|galley| galley.size().x).fold(0.0, f32::max);

    let label_galleys: Vec<_> = rows
        .iter()
        .map(|row| {
            let text = RichText::new(row.label());
            let text = if row.other { text.italics() } else { text };
            egui::WidgetText::from(text).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                width * MAX_LABEL_SHARE,
                TextStyle::Body,
            )
        })
        .collect();
    let label_width = label_galleys.iter().map(|galley| galley.size().x).fold(0.0, f32::max);
    let max_bar = (width - label_width - count_width - 2.0 * GAP).max(0.0);

    let visuals = ui.visuals().clone();
    for (index, row) in rows.iter().enumerate() {
        let (label, count) = (&label_galleys[index], &count_galleys[index]);
        let height = label.size().y.max(count.size().y) + 2.0 * ROW_PADDING;
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let summary = row.summary(total);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, true, format!("{}: {summary}", row.label()))
        });
        let hovered = response.hovered();
        response.on_hover_text(&summary);

        let painter = ui.painter_at(rect);
        let text_color = visuals.text_color();
        painter.galley(
            egui::pos2(rect.left(), rect.center().y - label.size().y / 2.0),
            label.clone(),
            text_color,
        );

        let radius = CornerRadius::same(BAR_RADIUS);
        let bar_left = rect.left() + label_width + GAP;
        let track = egui::Rect::from_min_size(
            egui::pos2(bar_left, rect.top() + ROW_PADDING),
            Vec2::new(max_bar, height - 2.0 * ROW_PADDING),
        );
        painter.rect_filled(track, radius, visuals.extreme_bg_color);

        let fill = if row.other {
            visuals.weak_text_color().gamma_multiply(0.6)
        } else {
            visuals.selection.bg_fill
        };
        let bar_width = bar_width(row.votes, max_votes, max_bar);
        if bar_width > 0.0 {
            let bar = egui::Rect::from_min_size(track.min, Vec2::new(bar_width, track.height()));
            painter.rect_filled(bar, radius, fill);
            if hovered {
                painter.rect_stroke(
                    bar,
                    radius,
                    Stroke::new(1.0, visuals.selection.stroke.color),
                    StrokeKind::Inside,
                );
            }
        }

        painter.galley(
            egui::pos2(rect.right() - count.size().x, rect.center().y - count.size().y / 2.0),
            count.clone(),
            text_color,
        );
    }
}

fn plain_galley(ui: &egui::Ui, text: &str, wrap_width: f32) -> std::sync::Arc<egui::Galley> {
    egui::WidgetText::from(text).into_galley(ui, Some(TextWrapMode::Extend), wrap_width, TextStyle::Body)
}

#[cfg(test)]
mod tests {
    use prono_api::{Answer, AnswerKind};
    use uuid::Uuid;

    use super::*;

    fn question() -> Question {
        Question {
            id: Uuid::nil(),
            text: "Background?".into(),
            kind: AnswerKind::Poll,
            possible_answers: vec!["C".into(), "C++".into()],
        }
    }

    fn vote(user: &str, text: &str) -> UserAnswer {
        UserAnswer {
            user: user.into(),
            answer: Answer::Poll(text.into()),
        }
    }

    #[test]
    fn rows_follow_the_tally_and_mark_other_answers() {
        let answers = [vote("a", "C"), vote("b", "Rust"), vote("c", "C"), vote("d", "Rust")];
        assert_eq!(
            rows(&question(), &answers),
            [
                Row {
                    text: "C".into(),
                    votes: 2,
                    other: false
                },
                Row {
                    text: "C++".into(),
                    votes: 0,
                    other: false
                },
                Row {
                    text: "Rust".into(),
                    votes: 2,
                    other: true
                },
            ]
        );
    }

    #[test]
    fn options_are_listed_without_answers() {
        let rows = rows(&question(), &[]);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.votes == 0 && !row.other));
    }

    #[test]
    fn other_rows_are_labelled() {
        let rows = rows(&question(), &[vote("a", "Rust")]);
        assert_eq!(rows[0].label(), "C");
        assert_eq!(rows[2].label(), "Rust (other)");
    }

    #[test]
    fn summary_shows_votes_and_share() {
        let row = |votes| Row {
            text: String::new(),
            votes,
            other: false,
        };
        assert_eq!(row(1).summary(4), "1 vote (25%)");
        assert_eq!(row(3).summary(4), "3 votes (75%)");
        assert_eq!(row(0).summary(0), "0 votes (0%)");
    }

    #[test]
    fn bars_are_proportional_to_the_most_voted() {
        assert!((bar_width(4, 4, 200.0) - 200.0).abs() < f32::EPSILON);
        assert!((bar_width(1, 4, 200.0) - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn no_votes_draw_no_bar() {
        assert!(bar_width(0, 4, 200.0).abs() < f32::EPSILON);
        assert!(bar_width(0, 0, 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_few_votes_stay_visible_and_within_the_space() {
        assert!((bar_width(1, 1000, 200.0) - MIN_BAR_WIDTH).abs() < f32::EPSILON);
        assert!(bar_width(1, 1, 2.0) <= 2.0);
    }

    #[test]
    fn other_is_selected_for_unknown_text_or_when_chosen() {
        let options = question().possible_answers;
        assert!(!other_selected(&options, "", false));
        assert!(!other_selected(&options, "C", false));
        assert!(other_selected(&options, "Rust", false));
        assert!(other_selected(&options, "", true));
    }
}
