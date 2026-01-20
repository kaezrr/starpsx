use eframe::egui;
use egui_extras::Column;

#[derive(PartialEq, Default)]
enum StateView {
    #[default]
    Cpu,
    Gpu,
    Irq,
    Sio0,
    Spu,
    Cdrom,
}

struct Breakpoint {
    address: u32,
    enabled: bool,
}

#[derive(Default)]
pub struct Debugger {
    breakpoints: Vec<Breakpoint>,
    state_view: StateView,
    disasm_pc: u32,
    address_input: String,
}

impl Debugger {
    pub fn show_ui(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("debug_left")
            .resizable(false)
            .show(ctx, |ui| {
                self.disassembly_view(ui);
            });

        egui::TopBottomPanel::bottom("debug_bottom").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                self.breakpoints_ui(ui);
                ui.separator();
                self.components_state_view(ui);
            })
        });
    }

    fn components_state_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state_view, StateView::Cpu, "CPU");
                ui.selectable_value(&mut self.state_view, StateView::Gpu, "GPU");
                ui.selectable_value(&mut self.state_view, StateView::Irq, "IRQ");
                ui.selectable_value(&mut self.state_view, StateView::Spu, "SPU");
                ui.selectable_value(&mut self.state_view, StateView::Sio0, "SIO0");
                ui.selectable_value(&mut self.state_view, StateView::Cdrom, "CDROM");
            });

            ui.separator();

            match self.state_view {
                StateView::Cpu => self.cpu_register_view(ui),
                _ => todo!("view not implemented"),
            }
        });
    }

    fn cpu_register_view(&self, ui: &mut egui::Ui) {
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

    fn disassembly_view(&self, ui: &mut egui::Ui) {
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
                                monospace_hex(ui, i * 4 + 0xfe100000);
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

    fn breakpoints_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_max_width(200.0);
            ui.horizontal(|ui| {
                ui.label("Add Breakpoint:");

                ui.add(
                    egui::TextEdit::singleline(&mut self.address_input)
                        .desired_width(90.0)
                        .hint_text("fe0c1234"),
                );

                if ui.button("Add").clicked() {
                    if let Ok(address) = u32::from_str_radix(&self.address_input, 16) {
                        self.breakpoints.push(Breakpoint {
                            address,
                            enabled: true,
                        });
                    }
                    self.address_input.clear();
                }
            });

            ui.separator();

            if self.breakpoints.is_empty() {
                ui.label("No breakpoints set...");
                return;
            }

            egui_extras::TableBuilder::new(ui)
                .id_salt("breakpoints")
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto())
                .column(Column::remainder())
                .column(Column::auto())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("");
                    });
                    header.col(|ui| {
                        ui.strong("Address");
                    });
                    header.col(|ui| {
                        ui.strong("");
                    });
                })
                .body(|mut body| {
                    let mut delete_index = None;
                    for (i, br) in self.breakpoints.iter_mut().enumerate() {
                        body.row(30.0, |mut row| {
                            row.col(|ui| {
                                ui.checkbox(&mut br.enabled, "");
                            });
                            row.col(|ui| {
                                monospace_hex(ui, br.address);
                            });
                            row.col(|ui| {
                                if ui.button("Delete").clicked() {
                                    delete_index = Some(i);
                                }
                            });
                        });
                    }

                    if let Some(i) = delete_index {
                        self.breakpoints.remove(i);
                    }
                });
        });
    }
}

fn monospace_hex(ui: &mut egui::Ui, val: u32) {
    ui.monospace(format!("0x{val:08x}"));
}
