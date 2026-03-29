use crate::HashMap;

use eframe::egui;

use egui_file::FileDialog;
use std::{
  ffi::OsStr,
  path::{Path, PathBuf},
};

use crate::compile;
use sid::ProgramValue;
use sid::TemplateValue;
use sid::DataValue;
use sid::GlobalState;
use sid::interpret_one;
use sid::get_built_in_functions;
use sid::ToSyntax;

/// Interpreter state stored flat to avoid lifetime issues with ExeState<'a>.
struct DebugState {
    program_stack: Vec<ProgramValue>,
    data_stack:    Vec<TemplateValue>,
    local_scope:   HashMap<String, DataValue>,
    scope_stack:   Vec<HashMap<String, DataValue>>,
    global_scope:  HashMap<String, DataValue>,
}

pub struct SidDebuggerApp {
    debug_state: Option<DebugState>,

    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

impl SidDebuggerApp {
    pub fn new() -> Self {
        Self {
            debug_state: None,
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
                    if let Some(s) = &mut self.debug_state {
                        let mut global_state = GlobalState::new(&mut s.global_scope);
                        interpret_one(
                            &mut s.data_stack,
                            &mut s.program_stack,
                            &mut s.local_scope,
                            &mut s.scope_stack,
                            &mut global_state,
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
                        self.debug_state = Some(DebugState {
                            program_stack: vec![ProgramValue::Invoke],
                            data_stack:    program.instructions,
                            local_scope:   HashMap::new(),
                            scope_stack:   Vec::new(),
                            global_scope:  program.global_scope,
                        });
                    }
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut painter_size = ui.available_size_before_wrap();
            if !painter_size.is_finite() {
                painter_size = egui::vec2(500.0, 500.0);
            }

            let Some(s) = self.debug_state.as_ref() else { return; };

            // Display stuff 
            ui.label(format!("File: {:?}", self.opened_file.as_ref().unwrap()));
            ui.horizontal(|ui| {
                
                ui.vertical(|ui| {
                    ui.label("program stack");
                    ui.label("-------------");
                    for (i, program_value) in s.program_stack.iter().enumerate() {
                        ui.label(format!("{}: {}", i, program_value.to_syntax()));
                    }
                });
                ui.vertical(|ui| {
                    for _data in 0..10 {
                        ui.label("|");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("data stack");
                    ui.label("-------------");
                    for (i, data) in s.data_stack.iter().enumerate() {
                        ui.label(format!("{}: {}", i, data.to_syntax()));
                    }
                });
                ui.vertical(|ui| {
                    for _data in 0..10 {
                        ui.label("|");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("local scope");
                    ui.label("-------------");
                    for (key, _data) in s.local_scope.iter() {
                        ui.label(format!("{:?}", key));
                    }
                });
                ui.vertical(|ui| {
                    for _data in 0..10 {
                        ui.label("|");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("global scope");
                    ui.label("-------------");
                    for (key, _data) in s.global_scope.iter() {
                        ui.label(format!("{:?}", key));
                    }
                });
            });
        });
    }
}

