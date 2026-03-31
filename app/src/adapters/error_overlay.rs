pub fn show_error_overlay(ctx: &egui::Context, error_message: &mut Option<String>) {
    if let Some(ref msg) = *error_message {
        let screen = ctx.content_rect();
        let msg = msg.clone();

        // Semi-transparent background covering the entire screen
        egui::Area::new(egui::Id::new("error_overlay_bg"))
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(160));
                if response.clicked() {
                    *error_message = None;
                }
            });

        // Centered error message on top
        egui::Area::new(egui::Id::new("error_overlay_msg"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Error");
                        ui.add_space(8.0);
                        ui.label(&msg);
                        ui.add_space(8.0);
                        #[cfg(debug_assertions)]
                        if ui.button("OK").clicked() {
                            *error_message = None;
                        }
                        #[cfg(not(debug_assertions))]
                        if ui.button("exit").clicked() {
                            std::process::exit(1);
                        }
                    });
                });
            });
    }
}
