use eframe::egui::{self, load::SizedTexture};
use egui_extras::Column;
use rfd::AsyncFileDialog;
use tracing::error;

use crate::{
    app::{Application, app_state::AppState, util::PendingDialog},
    config,
};

pub fn show_central_panel(app: &AppState, ctx: &egui::Context, vram_open: bool) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
        .show(ctx, |ui| {
            let (width, height) = if vram_open {
                (1024.0, 512.0)
            } else {
                (640.0, 480.0)
            };

            // No resolution means show a 4:3 black screen
            ui.centered_and_justified(|ui| {
                ui.add(
                    egui::Image::from_texture(SizedTexture::new(
                        &app.texture,
                        egui::vec2(width, height),
                    ))
                    .shrink_to_fit(),
                );
            });
        });
}

pub fn show_top_menu(app: &mut Application, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            egui::widgets::global_theme_preference_switch(ui);
            ui.separator();

            ui.menu_button("System", |ui| {
                // Only if a valid bios is set and emulator is not running
                ui.add_enabled_ui(
                    app.app_state.is_none() && app.app_config.bios_path.is_some(),
                    |ui| {
                        if ui.button("Start File").clicked() {
                            app.pending_dialog = Some(PendingDialog::SelectFile(Box::pin(
                                AsyncFileDialog::new()
                                    .add_filter("Game", &["bin", "BIN", "cue", "exe"])
                                    .set_title("Select file to Run")
                                    .pick_file(),
                            )));
                        }

                        if ui.button("Start BIOS").clicked() {
                            app.start_bios().unwrap_or_else(|err| {
                                error!(%err, "could not start bios");
                                app.toasts.error(format!("Could not start bios: {err}"));
                            })
                        }
                    },
                );

                // Only if emulator is running
                if let Some(emu) = app.app_state.take() {
                    let is_paused = emu.debugger.is_paused();
                    let label = if is_paused { "Resume" } else { "Pause" };

                    if ui.button(label).clicked() {
                        emu.debugger.toggle_pause();
                    }

                    if ui.button("Restart").clicked() {
                        emu.restart();
                    }

                    if ui.button("Stop").clicked() {
                        emu.shutdown();
                    } else {
                        app.app_state = Some(emu);
                    }
                }

                if ui.button("Exit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Settings", |ui| {
                if ui.button("BIOS Settings").clicked() {
                    app.bios_modal_open = true;
                }

                if ui.button("Keybinds").clicked() {
                    app.keybinds_table_open = true;
                }
            });

            ui.menu_button("Debug", |ui| {
                let label = if app.app_config.debugger_view {
                    "Close Debugger View"
                } else {
                    "Open Debugger View"
                };

                if ui.button(label).clicked() {
                    app.toggle_debugger_view();
                }

                let label = if app.vram_display_on() {
                    "Hide VRAM"
                } else {
                    "Show VRAM"
                };

                if ui.button(label).clicked() {
                    app.toggle_vram_display();
                }
            });

            ui.menu_button("Help", |ui| {
                ui.hyperlink_to("Source Code", "https://github.com/kaezrr/starpsx");
                ui.hyperlink_to(
                    "Report an Issue",
                    "https://github.com/kaezrr/starpsx/issues/new",
                );

                ui.separator();
                if ui.button("About StarPSX").clicked() {
                    app.info_modal_open = true;
                }
            });
        });
    });
}

pub fn show_info_modal(show_modal: &mut bool, ctx: &egui::Context) {
    if !*show_modal {
        return;
    }
    let modal = egui::Modal::new(egui::Id::new("Info")).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("About StarPSX");
        });

        ui.separator();
        ui.monospace(format!(
            "Version: {}-{}\nPlatform: {} {}",
            env!("CARGO_PKG_VERSION"),
            option_env!("GIT_HASH").unwrap_or("unknown"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        ));

        ui.separator();
        ui.label("StarPSX is a free and open source Playstation 1 emulator.");
        ui.label("It aims to be cross-platform and easy to use.");

        ui.separator();
        ui.monospace("Author: Anjishnu Banerjee <kaezr.dev@gmail.com>");

        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Source:");
            ui.hyperlink_to("Github", "https://github.com/kaezrr/starpsx");
            ui.label("License: GPLv3");
        });

        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            if ui.button("Close").clicked() {
                ui.close();
            }
        })
    });

    if modal.should_close() {
        *show_modal = false;
    }
}

pub fn show_performance_panel(app: &Application, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
        let m = app.get_metrics();
        ui.horizontal(|ui| {
            ui.label(format!("FPS: {}", m.fps));
            ui.separator();
            ui.label(format!("Core: {:.2} ms ({} FPS)", m.core_ms, m.core_fps));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label("Software Renderer");
                ui.separator();
                ui.label(match m.last_frame_data {
                    None => "Display Off".into(),
                    Some(([w, h], is_interlaced)) => {
                        if is_interlaced {
                            format!("{w}x{h} (Interlaced)")
                        } else {
                            format!("{w}x{h}")
                        }
                    }
                });
            })
        })
    });
}

pub fn show_bios_modal(app: &mut Application, ctx: &egui::Context) {
    if !app.bios_modal_open {
        return;
    }

    let modal = egui::Modal::new(egui::Id::new("Info")).show(ctx, |ui| {
        ui.set_width(400.0);
        ui.heading("Select BIOS image");
        ui.add_space(10.0);

        ui.label("Selected:");
        ui.horizontal_wrapped(|ui| match &app.app_config.bios_path {
            Some(path) => {
                ui.monospace(path.display().to_string());
            }
            None => {
                ui.colored_label(ui.visuals().error_fg_color, "No BIOS image selected");
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        egui::containers::Sides::new().show(
            ui,
            |ui| {
                if ui.button("Choose BIOS File…").clicked() {
                    app.pending_dialog = Some(PendingDialog::SelectBios(Box::pin(
                        AsyncFileDialog::new()
                            .add_filter("PlayStation BIOS", &["bin", "BIN"])
                            .set_title("Select PS1 BIOS image")
                            .pick_file(),
                    )));
                }
            },
            |ui| {
                if ui.button("Close").clicked() {
                    ui.close();
                }
            },
        );
    });

    if modal.should_close() {
        app.bios_modal_open = false;
    }
}

pub fn show_keybinds(open: &mut bool, ctx: &egui::Context) {
    egui::Window::new("Keybinds")
        .resizable(false)
        .collapsible(false)
        .default_pos(egui::pos2(30., 30.))
        .open(open)
        .show(ctx, |ui| {
            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .columns(Column::auto().at_least(100.0), 3)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Action");
                    });
                    header.col(|ui| {
                        ui.strong("Controller");
                    });
                    header.col(|ui| {
                        ui.strong("Keyboard");
                    });
                })
                .body(|body| {
                    body.rows(30.0, config::KEYBIND_ROWS.len(), |mut row| {
                        let i = row.index();
                        let keybind = &config::KEYBIND_ROWS[i];

                        row.col(|ui| {
                            ui.label(keybind.action);
                        });
                        row.col(|ui| {
                            ui.label(keybind.controller);
                        });
                        row.col(|ui| {
                            ui.label(keybind.keyboard);
                        });
                    });
                })
        });
}
