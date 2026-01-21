mod disasm;
pub mod snapshot;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, SyncSender};

use eframe::egui::{self, Align, Color32, RichText};
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

    pc_changed: bool,
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

            pc_changed: false,
        }
    }

    pub fn sync_send(&self, cmd: UiCommand) {
        self.input_tx.send(cmd).unwrap();
    }

    pub fn send(&self, cmd: UiCommand) {
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
        if !self.is_paused() {
            self.request_snapshot();
        }

        if let Ok(snapshot) = self.snapshot_rx.try_recv() {
            self.prev_snapshot = self.curr_snapshot.take();
            self.curr_snapshot = Some(snapshot);
            self.pc_changed = true;
        }

        egui::SidePanel::left("debug_left")
            .resizable(false)
            .min_width(400.0)
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

    fn request_snapshot(&self) {
        self.send(UiCommand::DebugRequestState);
    }

    fn components_state_view(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state_view, StateView::Cpu, "CPU");
                ui.add_enabled_ui(false, |ui| {
                    ui.selectable_value(&mut self.state_view, StateView::Gpu, "GPU");
                    ui.selectable_value(&mut self.state_view, StateView::Irq, "IRQ");
                    ui.selectable_value(&mut self.state_view, StateView::Spu, "SPU");
                    ui.selectable_value(&mut self.state_view, StateView::Sio0, "SIO0");
                    ui.selectable_value(&mut self.state_view, StateView::Cdrom, "CDROM");
                });
            });

            ui.separator();

            match self.state_view {
                StateView::Cpu => self.cpu_register_view(ui),
                _ => todo!("view not implemented"),
            }
        });
    }

    fn cpu_register_changed(&self, i: usize) -> bool {
        let Some(ref c) = self.curr_snapshot else {
            return false;
        };
        let Some(ref p) = self.prev_snapshot else {
            return false;
        };

        match i {
            32 => c.hi != p.hi,
            33 => c.lo != p.lo,
            34 => c.pc != p.pc,
            i => c.cpu_regs[i] != p.cpu_regs[i],
        }
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
            .body(|body| {
                let regs = snapshot.get_cpu_state();
                let rows = regs.len().div_ceil(2);

                body.rows(20.0, rows, |mut row| {
                    let i = row.index() * 2;

                    if let Some(&(name, val)) = regs.get(i) {
                        row.col(|ui| {
                            ui.monospace(name);
                        });
                        row.col(|ui| {
                            if self.cpu_register_changed(i) {
                                monospace_hex_change(ui, val);
                            } else {
                                monospace_hex(ui, val, true);
                            }
                        });
                    }

                    if let Some(&(name, val)) = regs.get(i + 1) {
                        row.col(|ui| {
                            ui.monospace(name);
                        });
                        row.col(|ui| {
                            if self.cpu_register_changed(i + 1) {
                                monospace_hex_change(ui, val);
                            } else {
                                monospace_hex(ui, val, true);
                            }
                        });
                    }
                });
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

            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("disasm")
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(10.0))
                .column(Column::exact(90.0))
                .column(Column::exact(80.0))
                .column(Column::remainder())
                .animate_scrolling(false);

            // scroll to program counter
            if !is_paused || self.pc_changed {
                self.pc_changed = false;
                table = table.scroll_to_row(100, Some(Align::TOP));
            }

            table
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
                .body(|body| {
                    let diassembly = snapshot.get_disassembly();
                    let breakpoint_set: HashSet<u32> = self
                        .breakpoints
                        .iter()
                        .filter(|b| b.enabled)
                        .map(|b| b.address)
                        .collect();

                    body.rows(20.0, diassembly.len(), |mut row| {
                        let i = row.index();
                        let (addr, word, disasm) = &diassembly[i];

                        row.col(|ui| {
                            line_indicator(ui, snapshot.pc, *addr, &breakpoint_set);
                        });

                        row.col(|ui| {
                            monospace_hex(ui, *addr, true);
                        });

                        row.col(|ui| {
                            monospace_hex(ui, *word, false);
                        });

                        row.col(|ui| {
                            disasm.label_monospace(ui);
                        });
                    });
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
                        .hint_text("800FE234"),
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
                .body(|body| {
                    let mut actions = Vec::new();
                    body.rows(20.0, self.breakpoints.len(), |mut row| {
                        let i = row.index();
                        let br = &self.breakpoints[i];

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
    ui.monospace(format!("{}{val:08X}", if prefix { "0x" } else { "" }));
}

fn monospace_hex_change(ui: &mut egui::Ui, val: u32) {
    ui.label(
        egui::RichText::new(format!("0x{val:08X}"))
            .monospace()
            .color(egui::Color32::RED),
    );
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

fn line_indicator(ui: &mut egui::Ui, pc: u32, addr: u32, breakpoint_set: &HashSet<u32>) {
    let is_dark = ui.visuals().dark_mode;

    let (label, color) = if pc == addr {
        let color = if is_dark {
            Color32::from_rgb(255, 220, 0)
        } else {
            Color32::from_rgb(200, 140, 0)
        };
        ("→", color) // Simple and clean
    } else if breakpoint_set.contains(&addr) {
        let color = if is_dark {
            Color32::from_rgb(255, 80, 80)
        } else {
            Color32::from_rgb(180, 0, 0)
        };
        ("●", color)
    } else {
        ("", Color32::TRANSPARENT)
    };

    ui.label(RichText::new(label).monospace().color(color));
}
