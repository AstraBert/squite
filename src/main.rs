mod dbops;

use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender, channel},
};

use eframe::egui;
use egui_extras::{Column, TableBuilder};
use serde_json::Value;

use crate::dbops::execute_sqlite_query;

struct SquiteApp {
    database_file: String,
    query: String,
    query_result: Option<HashMap<String, Vec<Value>>>,
    sx: Sender<HashMap<String, Vec<Value>>>,
    rx: Receiver<HashMap<String, Vec<Value>>>,
    loading: bool,
}

impl Default for SquiteApp {
    fn default() -> Self {
        let (sx, rx) = channel();
        Self {
            database_file: "".to_owned(),
            query: "".to_owned(),
            sx,
            rx,
            query_result: None,
            loading: false,
        }
    }
}

impl eframe::App for SquiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(result) = self.rx.try_recv() {
            self.query_result = Some(result);
            self.loading = false;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SqUIte");
            ui.hyperlink_to("GitHub Repository", "https://github.com/AstraBert/squite");
            ui.add(
                egui::Image::new(egui::include_image!("../assets/arxiv_cli.png"))
                    .max_width(200.0)
                    .corner_radius(10),
            );
            ui.horizontal(|ui| {
                let file_db_label = ui.label("Database File Path (absolute): ");
                ui.text_edit_singleline(&mut self.database_file)
                    .labelled_by(file_db_label.id);
            });
            ui.vertical(|ui| {
                let sqlite_query_label = ui.label("SQL Query:");
                ui.text_edit_multiline(&mut self.query)
                    .labelled_by(sqlite_query_label.id);
            });
            if ui.button("▶️ Run!").clicked() && !self.loading {
                if self.database_file.is_empty() || self.query.is_empty() {
                    ui.label("Both the database file path and the query should be non-empty");
                } else {
                    self.loading = true;
                    self.query_result = None;
                    let sx = self.sx.clone();
                    let ctx = ctx.clone();
                    let query = self.query.clone();
                    let db_file = self.database_file.clone();

                    std::thread::spawn(move || {
                        // Simulate slow work
                        let result = execute_sqlite_query(&query, &db_file).unwrap();

                        sx.send(result).unwrap();
                        ctx.request_repaint(); // wake up the UI when done
                    });
                }
            }
            if self.loading {
                ui.spinner();
                ctx.request_repaint();
            }
            if let Some(result) = &self.query_result {
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
                    table = table.column(Column::auto());
                    i += 1;
                }
                table
                    .header(20.0, |mut header| {
                        for column in result.keys() {
                            header.col(|ui| {
                                ui.strong(column);
                            });
                        }
                    })
                    .body(|mut body| {
                        for (_, arr) in result.iter() {
                            body.row(18.0, |mut row| {
                                for val in arr {
                                    row.col(|ui| {
                                        ui.label(val.to_string());
                                    });
                                }
                            })
                        }
                    });
            }
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
