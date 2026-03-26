mod dbops;

use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use indexmap::IndexMap;
use rfd::FileDialog;
use serde_json::Value;

use crate::dbops::{execute_sqlite_query, execute_sqlite_statement};

struct SquiteApp {
    database_file: String,
    query: String,
    query_result: Option<IndexMap<String, Vec<Value>>>,
    execution_result: Option<usize>,
    error_message: Option<String>,
    query_error: Option<String>,
    sx: Sender<IndexMap<String, Vec<Value>>>,
    rx: Receiver<IndexMap<String, Vec<Value>>>,
    sx_error: Sender<String>,
    rx_error: Receiver<String>,
    sx_exec: Sender<usize>,
    rx_exec: Receiver<usize>,
    loading: bool,
    allow_modifications: bool,
}

impl Default for SquiteApp {
    fn default() -> Self {
        let (sx, rx) = channel();
        let (sx_error, rx_error) = channel();
        let (sx_exec, rx_exec) = channel();
        Self {
            database_file: "".to_owned(),
            query: "".to_owned(),
            error_message: None,
            query_error: None,
            sx,
            rx,
            sx_error,
            rx_error,
            sx_exec,
            rx_exec,
            query_result: None,
            execution_result: None,
            loading: false,
            allow_modifications: false,
        }
    }
}

fn pick_database_file(current_path: &str) -> Option<String> {
    let current_path = Path::new(current_path);
    let mut dialog = FileDialog::new().set_title("Select SQLite database");

    if current_path.is_file() {
        if let Some(parent) = current_path.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(file_name) = current_path.file_name().and_then(|name| name.to_str()) {
            dialog = dialog.set_file_name(file_name);
        }
    } else if current_path.is_dir() {
        dialog = dialog.set_directory(current_path);
    }

    dialog
        .pick_file()
        .map(|path| path.as_path().display().to_string())
}

impl eframe::App for SquiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(result) = self.rx.try_recv() {
            self.query_result = Some(result);
            self.loading = false;
        }
        if let Ok(rows_count) = self.rx_exec.try_recv() {
            self.execution_result = Some(rows_count);
            self.loading = false;
        }
        if let Ok(err) = self.rx_error.try_recv() {
            self.query_error = Some(err);
            self.loading = false;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("SqUIte");
                ui.add_space(4.0);
                ui.hyperlink_to("GitHub Repository", "https://github.com/AstraBert/squite");
                ui.add_space(10.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../assets/squite.png"))
                        .max_width(350.0)
                        .corner_radius(10),
                );
                ui.add_space(20.0);
                ui.vertical_centered_justified(|ui| {
                    let file_db_label = ui.strong("Database File Path (absolute): ");
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let response = ui.add_sized(
                            [(ui.available_width() - 90.0).max(160.0), 0.0],
                            egui::TextEdit::singleline(&mut self.database_file),
                        );
                        response.labelled_by(file_db_label.id);

                        if ui.button("Browse...").clicked() {
                            if let Some(path) = pick_database_file(&self.database_file) {
                                self.database_file = path;
                                self.error_message = None;
                                self.query_error = None;
                            }
                        }
                    });
                });
                ui.add_space(10.0);
                ui.vertical_centered_justified(|ui| {
                    let sqlite_query_label = ui.strong("SQL Query:");
                    ui.add_space(4.0);
                    ui.text_edit_multiline(&mut self.query)
                        .labelled_by(sqlite_query_label.id);
                });
                ui.add_space(4.0);
                ui.checkbox(&mut self.allow_modifications, "This query modifies data")
                    .on_hover_text("Check if you want to run INSERT, UPDATE, DELETE or other statements that modify the database");
                if ui
                    .button(RichText::new("Run!").color(Color32::LIGHT_GREEN))
                    .clicked()
                    && !self.loading
                {
                    self.query_error = None;
                    self.error_message = None;
                    if self.database_file.is_empty() || self.query.is_empty() {
                        self.error_message = Some(
                            "Both the database file path and the query should be non-empty"
                                .to_string(),
                        );
                    } else {
                        self.loading = true;
                        self.query_result = None;
                        self.execution_result = None;
                        let sx = self.sx.clone();
                        let sx_exec = self.sx_exec.clone();
                        let sx_err = self.sx_error.clone();
                        let ctx = ctx.clone();
                        let query = self.query.clone();
                        let db_file = self.database_file.clone();

                        if !self.allow_modifications {
                            std::thread::spawn(move || {
                                match execute_sqlite_query(&query, &db_file) {
                                    Ok(result) => {
                                        sx.send(result).unwrap();
                                    }
                                    Err(err) => {
                                        sx_err.send(err.to_string()).unwrap();
                                    }
                                }
                                ctx.request_repaint(); // wake up the UI when done
                            });
                        } else {
                            std::thread::spawn(move || {
                                match execute_sqlite_statement(&query, &db_file) {
                                    Ok(result) => {
                                        sx_exec.send(result).unwrap();
                                    }
                                    Err(err) => {
                                        match err {
                                            rusqlite::Error::ExecuteReturnedResults => {
                                                if query.to_lowercase().starts_with("select") {
                                                    sx_err.send("You cannot execute a SELECT statement with the data modification checkbox checked.".to_string()).unwrap();
                                                } else {
                                                    sx_err.send(err.to_string()).unwrap();
                                                }
                                            },
                                            _ => {
                                                sx_err.send(err.to_string()).unwrap();
                                            }
                                        }
                                    }
                                }
                                ctx.request_repaint(); // wake up the UI when done
                            });
                        }
                    }
                }
                if ui
                    .button(RichText::new("Clear").color(Color32::LIGHT_RED))
                    .clicked()
                {
                    self.query_result = None;
                    self.query_error = None;
                    self.error_message = None;
                    self.execution_result = None;
                }
                if let Some(err) = &self.error_message {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, err);
                }
                if let Some(query_err) = &self.query_error {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, query_err);
                }
                if self.loading {
                    ui.add_space(4.0);
                    ui.spinner();
                    ctx.request_repaint();
                }
                if let Some(exec_result) = &self.execution_result {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::LIGHT_GREEN, format!("Success. {:?} row(s) affected.", exec_result));
                }
                if let Some(result) = &self.query_result {
                    ui.add_space(8.0);
                    let available_height = ui.available_height();
                    let mut table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .sense(egui::Sense::click())
                        .min_scrolled_height(0.0)
                        .max_scroll_height(available_height);
                    let mut i = 0;
                    while i < result.len() {
                        table = table.column(Column::remainder().at_least(20.0).clip(true));
                        i += 1;
                    }
                    let num_rows = result.values().next().map(|v| v.len()).unwrap_or(0);
                    let keys: Vec<&String> = result.keys().collect();
                    table
                        .header(20.0, |mut header| {
                            for column in result.keys() {
                                header.col(|ui| {
                                    ui.strong(column);
                                });
                            }
                        })
                        .body(|mut body| {
                            for row_idx in 0..num_rows {
                                body.row(18.0, |mut row| {
                                    for key in &keys {
                                        row.col(|ui| {
                                            let val = &result[*key][row_idx];
                                            ui.label(val.to_string());
                                        });
                                    }
                                });
                            }
                        });
                }
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        ..Default::default()
    };
    eframe::run_native(
        "SqUIte",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<SquiteApp>::default())
        }),
    )
}
