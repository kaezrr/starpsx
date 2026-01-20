use eframe::egui;
use egui_extras::Column;

#[derive(PartialEq)]
enum StateView {
    Cpu,
    Gpu,
    Irq,
    Sio0,
    Spu,
    Cdrom,
}

pub fn show_debug_ui(ctx: &egui::Context) {
    egui::SidePanel::left("debug_left")
        .resizable(false)
        .show(ctx, |ui| {
            disassembly_view(ui);
        });

    egui::TopBottomPanel::bottom("debug_bottom").show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            breakpoints_ui(ui);

            ui.separator();

            components_state_view(ui);
        })
    });
}

fn components_state_view(ui: &mut egui::Ui) {
    let mut curr_view = StateView::Cpu;
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut curr_view, StateView::Cpu, "CPU");
            ui.selectable_value(&mut curr_view, StateView::Gpu, "GPU");
            ui.selectable_value(&mut curr_view, StateView::Irq, "IRQ");
            ui.selectable_value(&mut curr_view, StateView::Spu, "SPU");
            ui.selectable_value(&mut curr_view, StateView::Sio0, "SIO0");
            ui.selectable_value(&mut curr_view, StateView::Cdrom, "CDROM");
        });

        ui.separator();

        match curr_view {
            StateView::Cpu => cpu_register_view(ui),
            _ => todo!("view not implemented"),
        }
    });
}

fn cpu_register_view(ui: &mut egui::Ui) {
    egui_extras::TableBuilder::new(ui)
        .id_salt("cpu_state")
        .striped(true)
        .resizable(false)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::auto())
        .column(Column::remainder())
        .column(Column::auto())
        .column(Column::remainder())
        .header(20.0, |mut header| {
            header.col(|ui| {
                ui.strong("Register");
            });
            header.col(|ui| {
                ui.strong("Value");
            });
            header.col(|ui| {
                ui.strong("Register");
            });
            header.col(|ui| {
                ui.strong("Value");
            });
        })
        .body(|mut body| {
            for i in (0..32).step_by(2) {
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.monospace(format!("reg{i:02}"));
                    });
                    row.col(|ui| {
                        ui.monospace("0x00000000");
                    });
                    row.col(|ui| {
                        ui.monospace(format!("reg{:02}", i + 1));
                    });
                    row.col(|ui| {
                        ui.monospace("0x00000000");
                    });
                });
            }
        });
}

fn disassembly_view(ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui.button("Pause").clicked() {}
            if ui.button("Step").clicked() {}
        });

        ui.separator();

        egui_extras::TableBuilder::new(ui)
            .id_salt("disasm")
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::exact(90.0))
            .column(Column::exact(80.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("");
                });
                header.col(|ui| {
                    ui.strong("Address");
                });
                header.col(|ui| {
                    ui.strong("Word");
                });
                header.col(|ui| {
                    ui.strong("Instruction");
                });
            })
            .body(|mut body| {
                for i in 0..32 {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            ui.monospace(if i == 3 { "⏵ " } else { "" });
                        });
                        row.col(|ui| {
                            ui.monospace(format!("0x{:08x}", i * 4 + 0x7000));
                        });
                        row.col(|ui| {
                            ui.monospace("00000000");
                        });
                        row.col(|ui| {
                            ui.monospace("sll 0, 0");
                        });
                    });
                }
            });
    });
}

fn breakpoints_ui(ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.set_max_width(200.0);
        ui.horizontal(|ui| {
            ui.label("Add Breakpoint:");

            let mut addr_input = String::new();
            ui.add(
                egui::TextEdit::singleline(&mut addr_input)
                    .desired_width(90.0)
                    .hint_text("0xF8971000"),
            );

            if ui.button("Add").clicked() {}
        });

        ui.separator();

        egui_extras::TableBuilder::new(ui)
            .id_salt("breakpoints")
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("");
                });
                header.col(|ui| {
                    ui.strong("Address");
                });
            })
            .body(|mut body| {
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.checkbox(&mut true, "");
                    });
                    row.col(|ui| {
                        ui.monospace("0x00000000");
                    });
                });
            });
    });
}
