pub fn show_ui(ctx: &egui::Context, _system: &starpsx_core::System) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        // The top panel is often a good place for a menu bar:

        egui::MenuBar::new().ui(ui, |ui| {
            // NOTE: no File->Quit on web pages!
            let is_web = cfg!(target_arch = "wasm32");
            if !is_web {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);
            }

            egui::widgets::global_theme_preference_buttons(ui);
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        // The central panel the region left after adding TopPanel's and SidePanel's
        ui.heading("eframe template");

        ui.horizontal(|ui| {
            ui.label("Write something: ");
        });

        ui.separator();

        ui.add(egui::github_link_file!(
            "https://github.com/emilk/eframe_template/blob/main/",
            "Source code."
        ));

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            egui::warn_if_debug_build(ui);
        });
    });
}
