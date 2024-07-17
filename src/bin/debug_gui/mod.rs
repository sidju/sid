use crate::HashMap;

use eframe::egui;

use egui_file::FileDialog;
use std::{
  ffi::OsStr,
  path::{Path, PathBuf},
};

use crate::compile;
use crate::Program;
use sid::ExeState;
use sid::ProgramValue;
use sid::interpret_one;
use sid::get_built_in_functions;
use sid::ToSyntax;

pub struct SidDebuggerApp {
    exe_state: Option<ExeState>,

    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

impl SidDebuggerApp {
    pub fn new() -> Self {
        Self {
            exe_state: None,
            opened_file: None,
            open_file_dialog: None,
        }
    }
}

impl eframe::App for SidDebuggerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.interact_size.y *= 1.4;
                ui.style_mut()
                    .text_styles
                    .get_mut(&egui::TextStyle::Button)
                    .unwrap()
                    .size *= 1.4;

                if (ui.button("Open")).clicked() {
                    // Show only files with the extension "sid".
                    let filter = Box::new({
                        let ext = Some(OsStr::new("sid"));
                        move |path: &Path| -> bool { path.extension() == ext }
                    });
                    let mut dialog = FileDialog::open_file(self.opened_file.clone()).show_files_filter(filter);
                    dialog.open();
                    self.open_file_dialog = Some(dialog);
                }

                if ui.button("Reset").clicked() {
                    *self = Self::new();
                }
                if ui.button("Step").clicked() || ui.input(|i| i.key_pressed(egui::Key::F10)) {
                    if let Some(exe_state) = &mut self.exe_state {
                        interpret_one(
                            exe_state,
                            &get_built_in_functions()
                        );
                    }
                }
            });

            if let Some(dialog) = &mut self.open_file_dialog {
                if dialog.show(ctx).selected() {
                    if let Some(file) = dialog.path() {
                        self.opened_file = Some(file.to_path_buf());
                        let file_content = std::fs::read_to_string(self.opened_file.as_ref().unwrap())
                            .expect("Failed to read file");

                        let program = compile(&file_content);
                        let exe_state = ExeState {
                            program_stack: vec![ProgramValue::Invoke],
                            data_stack: program.instructions,
                            local_scope: HashMap::new(),
                            global_scope: program.global_scope,
                        };
                        self.exe_state = Some(exe_state);
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut painter_size = ui.available_size_before_wrap();
            if !painter_size.is_finite() {
                painter_size = egui::vec2(500.0, 500.0);
            }

            if self.exe_state.is_none() {
                return;
            }

            // Display stuff 
            ui.label(format!("File: {:?}", self.opened_file.as_ref().unwrap()));
            ui.horizontal(|ui| {
                
                ui.vertical(|ui| {
                    ui.label("program stack");
                    ui.label("-------------");
                    let exe_state = self.exe_state.as_ref().unwrap();
                    for (i, program_value) in exe_state.program_stack.iter().enumerate() {
                        ui.label(format!("{}: {}", i, program_value.to_syntax()));
                    }
                });
                ui.vertical(|ui| {
                    for data in 0..10 {
                        ui.label("|");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("data stack");
                    ui.label("-------------");
                    let exe_state = self.exe_state.as_ref().unwrap();
                    for (i, data) in exe_state.data_stack.iter().enumerate() {
                        ui.label(format!("{}: {}", i, data.to_syntax()));
                    }
                });
                ui.vertical(|ui| {
                    for data in 0..10 {
                        ui.label("|");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("local scope");
                    ui.label("-------------");
                    let exe_state = self.exe_state.as_ref().unwrap();
                    for (key, _data) in exe_state.local_scope.iter() {
                        ui.label(format!("{:?}", key));
                    }
                });
                ui.vertical(|ui| {
                    for data in 0..10 {
                        ui.label("|");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("global scope");
                    ui.label("-------------");
                    let exe_state = self.exe_state.as_ref().unwrap();
                    for (key, _data) in exe_state.global_scope.iter() {
                        ui.label(format!("{:?}", key));
                    }
                });
            });
        });
    }
}

