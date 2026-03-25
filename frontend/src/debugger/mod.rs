mod disasm;
pub mod snapshot;

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;

use eframe::egui::Align;
use eframe::egui::Color32;
use eframe::egui::RichText;
use eframe::egui::{self};
use egui_extras::Column;
use starpsx_core::SystemSnapshot;

use crate::emulator::SharedState;
use crate::emulator::UiCommand;

pub struct Debugger {
    breakpoints: Vec<Breakpoint>,
    state_view: StateView,
    address_input: String,

    shared_state: Arc<SharedState>,
    input_tx: SyncSender<UiCommand>,
    snapshot_rx: Receiver<SystemSnapshot>,

    prev_snapshot: Option<SystemSnapshot>,
    curr_snapshot: Option<SystemSnapshot>,

    pc_changed: bool,
}

impl Debugger {
    pub fn new(
        shared_state: Arc<SharedState>,
        input_tx: SyncSender<UiCommand>,
        snapshot_rx: Receiver<SystemSnapshot>,
    ) -> Self {
        Self {
            shared_state,
            input_tx,
            snapshot_rx,

            breakpoints: Vec::default(),
            state_view: StateView::default(),
            address_input: String::default(),
            prev_snapshot: None,
            curr_snapshot: None,

            pc_changed: false,
        }
    }

    pub fn sync_send(&self, cmd: UiCommand) {
        self.input_tx.send(cmd).expect("send to ui channel");
    }

    pub fn send(&self, cmd: UiCommand) {
        let _ = self.input_tx.try_send(cmd);
    }

    pub fn is_paused(&self) -> bool {
        self.shared_state.is_paused()
    }

    pub fn toggle_pause(&self) {
        if self.shared_state.is_paused() {
            self.shared_state.resume();
        } else {
            self.shared_state.pause();
        }
    }

    pub fn restart(&self) {
        self.sync_send(UiCommand::Restart);
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

        egui::SidePanel::left("debug_view")
            .width_range(500.0..=800.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    self.components_state_view(ui);
                });
            });
    }

    fn request_snapshot(&self) {
        self.send(UiCommand::DebugRequestState);
    }

    fn components_state_view(&mut self, ui: &mut egui::Ui) {
        let mut state_view = self.state_view;
        egui::Sides::new().show(
            ui,
            |ui| {
                ui.selectable_value(&mut state_view, StateView::Cpu, "CPU");
                ui.selectable_value(&mut state_view, StateView::Spu, "SPU");
                ui.selectable_value(&mut state_view, StateView::Gpu, "GPU");
            },
            |ui| {
                let is_paused = self.shared_state.is_paused();
                ui.add_enabled_ui(is_paused && self.state_view == StateView::Cpu, |ui| {
                    if ui.button("Step").clicked() {
                        self.sync_send(UiCommand::DebugStep);
                    }
                });

                if ui.button("Restart").clicked() {
                    self.restart();
                }

                let label = if is_paused { "Resume" } else { "Pause" };
                if ui.button(label).clicked() {
                    self.toggle_pause();
                }
            },
        );

        self.state_view = state_view;

        ui.separator();

        match self.state_view {
            StateView::Cpu => self.cpu_state_view(ui),
            StateView::Gpu => self.gpu_state_view(ui),
            StateView::Spu => self.spu_state_view(ui),
        }
    }

    fn cpu_state_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("disassembly_scroll")
            .max_height(ui.available_height() * 0.5)
            .show(ui, |ui| {
                self.disassembly_view(ui);
            });

        ui.separator();

        egui::ScrollArea::vertical()
            .id_salt("registers_scroll")
            .max_height(ui.available_height() - 220.0) // leave room for breakpoints
            .show(ui, |ui| {
                self.cpu_register_view(ui);
            });

        ui.separator();

        self.breakpoints_ui(ui);
    }

    const fn cpu_register_changed(&self, i: usize) -> bool {
        let Some(ref c) = self.curr_snapshot else {
            return false;
        };
        let Some(ref p) = self.prev_snapshot else {
            return false;
        };

        match i {
            32 => c.cpu.hi != p.cpu.hi,
            33 => c.cpu.lo != p.cpu.lo,
            34 => c.cpu.pc != p.cpu.pc,
            i => c.cpu.regs[i] != p.cpu.regs[i],
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
            .column(Column::remainder())
            .column(Column::remainder())
            .column(Column::remainder())
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
                let regs = snapshot::get_cpu_state(snapshot);
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

    fn spu_state_view(&self, ui: &mut egui::Ui) {
        let Some(ref snapshot) = self.curr_snapshot else {
            return;
        };
        let spu = &snapshot.spu;

        // Global SPU status
        ui.horizontal(|ui| {
            let (enabled_text, enabled_color) = if spu.enabled {
                ("ON", Color32::GREEN)
            } else {
                ("OFF", Color32::RED)
            };
            let (muted_text, muted_color) = if spu.muted {
                ("YES", Color32::RED)
            } else {
                ("NO", Color32::GREEN)
            };

            ui.label("Enabled:");
            ui.label(
                RichText::new(enabled_text)
                    .monospace()
                    .strong()
                    .color(enabled_color),
            );
            ui.separator();
            ui.label("Muted:");
            ui.label(
                RichText::new(muted_text)
                    .monospace()
                    .strong()
                    .color(muted_color),
            );
            ui.separator();
            ui.label("Vol L:");
            ui.monospace(format!("{:.1}%", spu.main_volume_left));
            ui.separator();
            ui.label("Vol R:");
            ui.monospace(format!("{:.1}%", spu.main_volume_right));
        });

        ui.separator();

        // Voice table
        let dim_color = if ui.visuals().dark_mode {
            Color32::from_gray(100)
        } else {
            Color32::from_gray(160)
        };

        egui_extras::TableBuilder::new(ui)
            .id_salt("spu_voices")
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto()) // #
            .column(Column::remainder()) // Phase
            .column(Column::remainder()) // Start
            .column(Column::remainder()) // Repeat
            .column(Column::remainder()) // Current
            .column(Column::remainder()) // Rate (Hz)
            .column(Column::remainder()) // Vol L
            .column(Column::remainder()) // Vol R
            .column(Column::remainder()) // ADSR Vol
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("#");
                });
                header.col(|ui| {
                    ui.strong("Phase");
                });
                header.col(|ui| {
                    ui.strong("Start");
                });
                header.col(|ui| {
                    ui.strong("Repeat");
                });
                header.col(|ui| {
                    ui.strong("Current");
                });
                header.col(|ui| {
                    ui.strong("Rate (Hz)");
                });
                header.col(|ui| {
                    ui.strong("Vol L");
                });
                header.col(|ui| {
                    ui.strong("Vol R");
                });
                header.col(|ui| {
                    ui.strong("ADSR Vol");
                });
            })
            .body(|body| {
                body.rows(20.0, 24, |mut row| {
                    let i = row.index();
                    let v = &spu.voices[i];
                    let is_off = v.adsr_phase == starpsx_core::AdsrPhase::Off;

                    let mono = |text: String| -> RichText {
                        let rt = RichText::new(text).monospace();
                        if is_off { rt.color(dim_color) } else { rt }
                    };

                    row.col(|ui| {
                        ui.label(mono(format!("{i:2}")));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("{}", v.adsr_phase)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("0x{:04X}", v.start_address)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("0x{:04X}", v.repeat_address)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("0x{:04X}", v.current_address)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("{:.1}", v.sample_rate)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("{:.1}%", v.volume_left)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("{:.1}%", v.volume_right)));
                    });
                    row.col(|ui| {
                        ui.label(mono(format!("{:.1}%", v.adsr_volume)));
                    });
                });
            });
    }

    fn gpu_state_view(&self, ui: &mut egui::Ui) {
        let Some(ref snapshot) = self.curr_snapshot else {
            return;
        };
        let gpu = &snapshot.gpu;

        let bool_str = |b: bool| if b { "Yes" } else { "No" };
        let coord = |p: (i32, i32)| format!("({}, {})", p.0, p.1);

        let rows: Vec<(&str, String)> = vec![
            // Display
            ("", "Display".into()),
            (
                "Video Mode",
                match gpu.vmode {
                    starpsx_core::VMode::Ntsc => "NTSC".into(),
                    starpsx_core::VMode::Pal => "PAL".into(),
                },
            ),
            ("Display Depth", format!("{:?}", gpu.display_depth)),
            ("Display Disabled", bool_str(gpu.display_disabled).into()),
            ("Interlaced", bool_str(gpu.interlaced).into()),
            ("VRAM Start", coord(gpu.display_vram_start)),
            (
                "Display Size",
                format!("{}x{}", gpu.display_width, gpu.display_height),
            ),
            ("Hor Range", format!("{}", gpu.display_hor_range)),
            ("Ver Range", format!("{}", gpu.display_ver_range)),
            // Drawing Area
            ("", "Drawing Area".into()),
            ("Top Left", coord(gpu.drawing_area_top_left)),
            ("Bottom Right", coord(gpu.drawing_area_bottom_right)),
            ("Offset", coord(gpu.drawing_area_offset)),
            // Texturing
            ("", "Texturing".into()),
            ("Dithering", bool_str(gpu.dithering).into()),
            (
                "Semi-Transparency",
                format!(
                    "({:.2}, {:.2})",
                    gpu.transparency_weights.0, gpu.transparency_weights.1
                ),
            ),
            ("Tex Window Mask", coord(gpu.texture_window_mask)),
            ("Tex Window Offset", coord(gpu.texture_window_offset)),
            // Masking
            ("", "Masking".into()),
            (
                "Preserve Masked",
                bool_str(gpu.preserve_masked_pixels).into(),
            ),
            (
                "Force Set Mask Bit",
                bool_str(gpu.force_set_masked_bit).into(),
            ),
            // Counters
            ("", "Counters".into()),
            ("Frame", format!("{}", gpu.frame_counter)),
            ("Line", format!("{}", gpu.line_counter)),
        ];

        egui_extras::TableBuilder::new(ui)
            .id_salt("gpu_state")
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::remainder())
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Property");
                });
                header.col(|ui| {
                    ui.strong("Value");
                });
            })
            .body(|body| {
                body.rows(20.0, rows.len(), |mut row| {
                    let (label, value) = &rows[row.index()];
                    let is_section = label.is_empty();

                    row.col(|ui| {
                        if is_section {
                            ui.strong(value.as_str());
                        } else {
                            ui.label(*label);
                        }
                    });
                    row.col(|ui| {
                        if !is_section {
                            ui.monospace(value.as_str());
                        }
                    });
                });
            });
    }

    fn disassembly_view(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.curr_snapshot.take() else {
            return;
        };
        let is_paused = self.shared_state.is_paused();
        ui.vertical(|ui| {
            // Define highlight color based on theme
            let item_spacing = ui.spacing().item_spacing;
            let pc_highlight = if ui.visuals().dark_mode {
                Color32::from_rgb(60, 60, 0) // Dark yellow tint
            } else {
                Color32::from_rgb(255, 250, 200) // Light yellow tint
            };

            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt("disasm")
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().at_least(10.0))
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::remainder())
                .animate_scrolling(false);

            // scroll to program counter
            if !is_paused || self.pc_changed {
                self.pc_changed = false;
                table = table.scroll_to_row(100, Some(Align::Center));
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
                    let diassembly = snapshot::get_disassembly(&snapshot);
                    let breakpoint_set: HashSet<u32> = self
                        .breakpoints
                        .iter()
                        .filter(|b| b.enabled)
                        .map(|b| b.address)
                        .collect();

                    body.rows(20.0, diassembly.len(), |mut row| {
                        let i = row.index();
                        let (addr, word, disasm) = &diassembly[i];
                        let is_current_pc = snapshot.cpu.pc == *addr;

                        let paint_bg = |ui: &mut egui::Ui| {
                            let gapless_rect = ui.max_rect().expand2(0.5 * item_spacing);
                            ui.painter().rect_filled(gapless_rect, 0.0, pc_highlight);
                        };

                        row.col(|ui| {
                            if is_current_pc {
                                paint_bg(ui);
                            }
                            line_indicator(ui, snapshot.cpu.pc, *addr, &breakpoint_set);
                        });
                        row.col(|ui| {
                            if is_current_pc {
                                paint_bg(ui);
                            }
                            monospace_hex(ui, *addr, true);
                        });
                        row.col(|ui| {
                            if is_current_pc {
                                paint_bg(ui);
                            }
                            monospace_hex(ui, *word, false);
                        });
                        row.col(|ui| {
                            if is_current_pc {
                                paint_bg(ui);
                            }
                            disasm.label_monospace(ui);
                        });
                    });
                });
        });
        self.curr_snapshot = Some(snapshot);
    }

    fn breakpoints_ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_min_height(100.0);
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
                .column(Column::remainder())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("Enabled");
                    });
                    header.col(|ui| {
                        ui.strong("Address");
                    });
                    header.col(|ui| {
                        ui.strong("Action");
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
                            }
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

#[derive(PartialEq, Default, Clone, Copy)]
enum StateView {
    #[default]
    Cpu,
    Spu,
    Gpu,
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
