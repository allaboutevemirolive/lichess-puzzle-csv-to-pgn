use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use eframe::egui;
use lichess_puzzle_csv_to_pgn::{
    ConversionError, ConversionProgress, ConversionStats, convert_csv_to_pgn_with_progress,
};

const APP_TITLE: &str = "Lichess Puzzle CSV to PGN";
const BRAND: egui::Color32 = egui::Color32::from_rgb(35, 105, 82);
const TEXT: egui::Color32 = egui::Color32::from_rgb(33, 43, 54);
const MUTED: egui::Color32 = egui::Color32::from_rgb(105, 115, 125);
const CANVAS: egui::Color32 = egui::Color32::from_rgb(250, 250, 250);

enum WorkerMessage {
    Progress(ConversionProgress),
    Complete(Result<ConversionStats, ConversionError>),
}

struct ConverterApp {
    input_path: String,
    output_path: String,
    minimum_rating: String,
    progress: Option<ConversionProgress>,
    status: String,
    error: bool,
    receiver: Option<Receiver<WorkerMessage>>,
}

impl Default for ConverterApp {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_path: String::new(),
            minimum_rating: "0".to_owned(),
            progress: None,
            status: String::new(),
            error: false,
            receiver: None,
        }
    }
}

impl ConverterApp {
    fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        configure_theme(&creation_context.egui_ctx);
        Self::default()
    }

    fn is_converting(&self) -> bool {
        self.receiver.is_some()
    }

    fn is_ready_to_convert(&self) -> bool {
        !self.is_converting()
            && !self.input_path.trim().is_empty()
            && !self.output_path.trim().is_empty()
            && self.minimum_rating.trim().parse::<u32>().is_ok()
    }

    fn choose_input(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CSV files", &["csv"])
            .set_title("Choose the Lichess puzzle CSV")
            .pick_file()
        else {
            return;
        };

        self.input_path = path.display().to_string();
        if self.output_path.is_empty() {
            let filename = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| format!("{stem}-filtered.pgn"))
                .unwrap_or_else(|| "lichess-puzzles-filtered.pgn".to_owned());
            self.output_path = path.with_file_name(filename).display().to_string();
        }
        self.status = "Puzzle database selected.".to_owned();
        self.error = false;
    }

    fn choose_output(&mut self) {
        let suggested_name = PathBuf::from(&self.output_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("lichess-puzzles-filtered.pgn")
            .to_owned();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PGN files", &["pgn"])
            .set_file_name(&suggested_name)
            .set_title("Save converted PGN as")
            .save_file()
        else {
            return;
        };
        self.output_path = path.display().to_string();
        self.status = "Output location selected.".to_owned();
        self.error = false;
    }

    fn start_conversion(&mut self) {
        let input = PathBuf::from(self.input_path.trim());
        let output = PathBuf::from(self.output_path.trim());
        if self.input_path.trim().is_empty() || !input.is_file() {
            self.fail("Choose an existing Lichess puzzle CSV file first.");
            return;
        }
        if self.output_path.trim().is_empty() {
            self.fail("Choose where to save the PGN file first.");
            return;
        }
        let minimum_rating = match self.minimum_rating.trim().parse::<u32>() {
            Ok(rating) => rating,
            Err(_) => {
                self.fail("Minimum rating must be a non-negative whole number.");
                return;
            }
        };

        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.progress = Some(ConversionProgress::default());
        self.status = "Converting…".to_owned();
        self.error = false;

        thread::spawn(move || {
            let progress_sender = sender.clone();
            let result = convert_csv_to_pgn_with_progress(
                &input,
                &output,
                minimum_rating,
                move |progress| {
                    let _ = progress_sender.send(WorkerMessage::Progress(progress));
                },
            );
            let _ = sender.send(WorkerMessage::Complete(result));
        });
    }

    fn poll_worker(&mut self) {
        let mut complete = None;
        if let Some(receiver) = &self.receiver {
            while let Ok(message) = receiver.try_recv() {
                match message {
                    WorkerMessage::Progress(progress) => self.progress = Some(progress),
                    WorkerMessage::Complete(result) => complete = Some(result),
                }
            }
        }
        if let Some(result) = complete {
            self.receiver = None;
            match result {
                Ok(stats) => {
                    self.progress = Some(ConversionProgress {
                        records_read: stats.records_read,
                        puzzles_written: stats.puzzles_written,
                    });
                    self.status = format!(
                        "Finished: {} puzzles exported from {} rows.",
                        stats.puzzles_written, stats.records_read
                    );
                    self.error = false;
                }
                Err(error) => self.fail(format!("Conversion failed: {error}")),
            }
        }
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.error = true;
    }

    fn show_converter(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Convert a Lichess puzzle database")
                .size(22.0)
                .strong()
                .color(TEXT),
        );
        ui.label(
            egui::RichText::new("Choose a CSV, destination, and minimum rating.").color(MUTED),
        );
        ui.add_space(18.0);

        let is_converting = self.is_converting();
        let mut choose_input = false;
        let mut choose_output = false;
        egui::Grid::new("conversion_form")
            .num_columns(3)
            .spacing(egui::vec2(10.0, 12.0))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Puzzle CSV");
                });
                ui.add_enabled(
                    !is_converting,
                    egui::TextEdit::singleline(&mut self.input_path)
                        .desired_width(340.0)
                        .hint_text("Select an unpacked Lichess .csv file"),
                );
                choose_input = ui
                    .add_enabled(!is_converting, egui::Button::new("Browse…"))
                    .clicked();
                ui.end_row();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Output PGN");
                });
                ui.add_enabled(
                    !is_converting,
                    egui::TextEdit::singleline(&mut self.output_path)
                        .desired_width(340.0)
                        .hint_text("Choose where to save the PGN"),
                );
                choose_output = ui
                    .add_enabled(!is_converting, egui::Button::new("Choose…"))
                    .clicked();
                ui.end_row();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Minimum rating");
                });
                ui.add_enabled(
                    !is_converting,
                    egui::TextEdit::singleline(&mut self.minimum_rating)
                        .desired_width(96.0)
                        .horizontal_align(egui::Align::Center),
                );
                ui.label(egui::RichText::new("and above").color(MUTED));
                ui.end_row();
            });
        if choose_input {
            self.choose_input();
        }
        if choose_output {
            self.choose_output();
        }

        if !self.minimum_rating.trim().is_empty()
            && self.minimum_rating.trim().parse::<u32>().is_err()
        {
            ui.add_space(6.0);
            ui.colored_label(
                egui::Color32::from_rgb(170, 46, 46),
                "Minimum rating must be a non-negative whole number.",
            );
        }

        if !self.status.is_empty() {
            ui.add_space(14.0);
            status_line(
                ui,
                &self.status,
                self.error,
                self.is_converting(),
                self.progress,
            );
        }

        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);
        if self.is_converting() {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled(false, egui::Button::new("Converting…"));
                ui.spinner();
            });
        } else {
            let is_ready = self.is_ready_to_convert();
            let clicked = ui
                .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_ready {
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("Convert to PGN")
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(BRAND)
                            .corner_radius(5)
                            .min_size(egui::vec2(130.0, 30.0)),
                        )
                        .clicked()
                    } else {
                        ui.add_enabled(false, egui::Button::new("Convert to PGN"))
                            .clicked()
                    }
                })
                .inner;
            if clicked {
                self.start_conversion();
            }
        }
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new("The input CSV is never modified.")
                .size(12.0)
                .color(MUTED),
        );
    }
}

impl eframe::App for ConverterApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_worker();
        if self.is_converting() {
            root_ui
                .ctx()
                .request_repaint_after(Duration::from_millis(100));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(CANVAS)
                    .inner_margin(egui::Margin::symmetric(30, 24)),
            )
            .show(root_ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_width(ui.available_width().min(640.0));
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        self.show_converter(ui)
                    });
                });
            });
    }
}

fn configure_theme(context: &egui::Context) {
    context.set_theme(egui::Theme::Light);
    let mut style = (*context.style_of(egui::Theme::Light)).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.interact_size = egui::vec2(40.0, 30.0);
    style.visuals.panel_fill = CANVAS;
    style.visuals.window_fill = CANVAS;
    style.visuals.text_edit_bg_color = Some(egui::Color32::WHITE);
    context.set_style_of(egui::Theme::Light, style);
}

fn status_line(
    ui: &mut egui::Ui,
    status: &str,
    is_error: bool,
    is_converting: bool,
    progress: Option<ConversionProgress>,
) {
    let color = if is_error {
        egui::Color32::from_rgb(170, 46, 46)
    } else if is_converting {
        egui::Color32::from_rgb(58, 101, 149)
    } else {
        BRAND
    };
    ui.horizontal(|ui| {
        if is_converting {
            ui.spinner();
        }
        ui.colored_label(color, status);
        if is_converting && let Some(progress) = progress {
            ui.label(
                egui::RichText::new(format!(
                    "{} rows read · {} puzzles written",
                    progress.records_read, progress.puzzles_written
                ))
                .color(MUTED),
            );
        }
    });
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 360.0])
            .with_min_inner_size([560.0, 310.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|creation_context| Ok(Box::new(ConverterApp::new(creation_context)))),
    )
}
