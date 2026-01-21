mod disasm;
pub mod snapshot;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender};

use eframe::egui::{self, Align, RichText};
use egui_extras::Column;

use crate::emulator::{SharedState, UiCommand};
use snapshot::DebugSnapshot;

pub struct Debugger {
    breakpoints: Vec<Breakpoint>,
    state_view: StateView,
    address_input: String,

    shared_state: Arc<SharedState>,
    input_tx: SyncSender<UiCommand>,
    snapshot_rx: Receiver<DebugSnapshot>,

    prev_snapshot: Option<DebugSnapshot>,
    curr_snapshot: Option<DebugSnapshot>,
}

impl Debugger {
    pub fn new(
        shared_state: Arc<SharedState>,
        input_tx: SyncSender<UiCommand>,
        snapshot_rx: Receiver<DebugSnapshot>,
    ) -> Self {
        Self {
            shared_state,
            input_tx,
            snapshot_rx,

            breakpoints: Default::default(),
            state_view: Default::default(),
            address_input: Default::default(),
            prev_snapshot: Default::default(),
            curr_snapshot: Default::default(),
        }
    }
    pub fn sync_send(&mut self, cmd: UiCommand) {
        self.input_tx.send(cmd).unwrap();
    }

    pub fn send(&mut self, cmd: UiCommand) {
        let _ = self.input_tx.try_send(cmd);
    }

    pub fn is_paused(&self) -> bool {
        self.shared_state.is_paused()
    }

    pub fn toggle_pause(&self) {
        match self.shared_state.is_paused() {
            true => self.shared_state.resume(),
            false => self.shared_state.pause(),
        };
    }

    pub fn load_metrics(&self) -> (f32, f32) {
        self.shared_state.load()
    }

    pub fn show_ui(&mut self, ctx: &egui::Context) {
        self.send(UiCommand::DebugRequestState);

        if let Ok(snapshot) = self.snapshot_rx.try_recv() {
            self.prev_snapshot = self.curr_snapshot.take();
            self.curr_snapshot = Some(snapshot);
        }

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
        let Some(ref snapshot) = self.curr_snapshot else {
            return;
        };

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
                let regs = snapshot.get_cpu_state();
                let (regs, rem) = regs.as_chunks::<2>();

                for r in regs {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.monospace(r[0].0);
                        });
                        row.col(|ui| {
                            monospace_hex(ui, r[0].1, true);
                        });
                        row.col(|ui| {
                            ui.monospace(r[1].0);
                        });
                        row.col(|ui| {
                            monospace_hex(ui, r[1].1, true);
                        });
                    });
                }

                for r in rem {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.monospace(r.0);
                        });
                        row.col(|ui| {
                            monospace_hex(ui, r.1, true);
                        });
                    });
                }
            });
    }

    fn disassembly_view(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.curr_snapshot.take() else {
            return;
        };
        let is_paused = self.shared_state.is_paused();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let label = if is_paused { "Resume" } else { "Pause" };

                if ui.button(label).clicked() {
                    self.toggle_pause();
                }

                ui.add_enabled_ui(is_paused, |ui| {
                    if ui.button("Step").clicked() {
                        self.sync_send(UiCommand::DebugStep);
                    }
                });
            });

            ui.separator();

            egui_extras::TableBuilder::new(ui)
                .id_salt("disasm")
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(10.0))
                .column(Column::exact(90.0))
                .column(Column::exact(80.0))
                .column(Column::auto())
                .animate_scrolling(false)
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
                    let diassembly = snapshot.get_disassembly();
                    let breakpoint_set: HashSet<u32> = self
                        .breakpoints
                        .iter()
                        .filter(|b| b.enabled)
                        .map(|b| b.address)
                        .collect();

                    for (addr, word, disasm) in diassembly {
                        let is_pc_row = snapshot.pc == addr;

                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                let response =
                                    ui.label(get_disasm_label(snapshot.pc, addr, &breakpoint_set));
                                if is_pc_row && !is_paused {
                                    response.scroll_to_me(Some(Align::Min));
                                }
                            });

                            row.col(|ui| {
                                monospace_hex(ui, addr, true);
                            });

                            row.col(|ui| {
                                monospace_hex(ui, word, false);
                            });

                            row.col(|ui| {
                                ui.monospace(disasm);
                            });
                        });
                    }
                });
        });

        self.curr_snapshot = Some(snapshot);
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

                if ui.button("Add").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Ok(address) = u32::from_str_radix(&self.address_input, 16) {
                        self.breakpoints.push(Breakpoint {
                            address,
                            enabled: true,
                        });
                        self.sync_send(UiCommand::DebugSetBreakpoint(address, true));
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
                    let mut actions = Vec::new();
                    for (i, br) in self.breakpoints.iter_mut().enumerate() {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                let mut enabled = br.enabled;
                                if ui.checkbox(&mut enabled, "").changed() {
                                    actions.push(BreakpointAction::Toggle { index: i, enabled });
                                };
                            });
                            row.col(|ui| {
                                monospace_hex(ui, br.address, true);
                            });
                            row.col(|ui| {
                                if ui.button("Delete").clicked() {
                                    actions.push(BreakpointAction::Delete { index: i });
                                }
                            });
                        });
                    }

                    for action in actions {
                        match action {
                            BreakpointAction::Toggle { index, enabled } => {
                                let addr = self.breakpoints[index].address;
                                self.breakpoints[index].enabled = enabled;
                                self.sync_send(UiCommand::DebugSetBreakpoint(addr, enabled));
                            }

                            BreakpointAction::Delete { index } => {
                                let addr = self.breakpoints[index].address;
                                self.sync_send(UiCommand::DebugSetBreakpoint(addr, false));
                                self.breakpoints.remove(index);
                            }
                        }
                    }
                });
        });
    }
}

fn monospace_hex(ui: &mut egui::Ui, val: u32, prefix: bool) {
    ui.monospace(format!("{}{val:08x}", if prefix { "0x" } else { "" }));
}

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

enum BreakpointAction {
    Toggle { index: usize, enabled: bool },
    Delete { index: usize },
}

fn get_disasm_label(pc: u32, addr: u32, breakpoint_set: &HashSet<u32>) -> RichText {
    let label = if pc == addr {
        "▶"
    } else if breakpoint_set.contains(&addr) {
        "●"
    } else {
        ""
    };

    let color = if pc == addr {
        egui::Color32::YELLOW
    } else {
        egui::Color32::RED
    };

    egui::RichText::new(label).monospace().color(color)
}
