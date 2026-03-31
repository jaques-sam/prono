use egui::{FontId, RichText};

pub fn render_footer(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(RichText::new("Made by Sam Jaques. ").font(FontId::proportional(8.0)));
        ui.label(RichText::new("Powered by ").font(FontId::proportional(8.0)));
        ui.hyperlink_to(
            RichText::new("egui").font(FontId::proportional(8.0)),
            "https://github.com/emilk/egui",
        );
        ui.label(RichText::new(" and ").font(FontId::proportional(8.0)));
        ui.hyperlink_to(
            RichText::new("eframe").font(FontId::proportional(8.0)),
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
