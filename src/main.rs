mod metadata;

use std::path::{Path, PathBuf};

use eframe::egui;
use metadata::ImageMetadata;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 680.0])
            .with_min_inner_size([640.0, 420.0])
            .with_title("Exif / XMP Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "Exif / XMP Viewer",
        options,
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::<MetadataApp>::default())
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "system_japanese".to_owned(),
        egui::FontData::from_static(include_bytes!("C:/Windows/Fonts/meiryo.ttc")).into(),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "system_japanese".to_owned());
    }
    ctx.set_fonts(fonts);
}

#[derive(Default)]
struct MetadataApp {
    current_path: Option<PathBuf>,
    metadata: Option<ImageMetadata>,
    error: Option<String>,
}

impl MetadataApp {
    fn open(&mut self, path: &Path) {
        self.current_path = Some(path.to_owned());
        self.error = None;
        self.metadata = None;

        if !is_supported_image(path) {
            self.error = Some("PNGまたはJPEG画像を指定してください。".to_owned());
            return;
        }

        match ImageMetadata::read(path) {
            Ok(metadata) => self.metadata = Some(metadata),
            Err(error) => self.error = Some(error),
        }
    }
}

impl eframe::App for MetadataApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped_files = ctx.input(|input| input.raw.dropped_files.clone());
        if let Some(path) = dropped_files.first().and_then(|file| file.path.as_deref()) {
            self.open(path);
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Exif / XMP Viewer");
                ui.add_space(12.0);
                if ui.button("画像を選択…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("PNG / JPEG", &["png", "jpg", "jpeg"])
                        .pick_file()
                    {
                        self.open(&path);
                    }
                }
            });
            if let Some(path) = &self.current_path {
                ui.label(path.display().to_string());
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::from_rgb(210, 70, 70), error);
            } else if let Some(metadata) = &self.metadata {
                ui.columns(2, |columns| {
                    show_exif(&mut columns[0], metadata);
                    show_xmp(&mut columns[1], metadata);
                });
            } else {
                let rect = ui.available_rect_before_wrap().shrink(24.0);
                let response = ui.allocate_rect(rect, egui::Sense::hover());
                let color = if response.hovered() {
                    egui::Color32::from_rgb(70, 110, 155)
                } else {
                    egui::Color32::from_gray(85)
                };
                ui.painter().rect_stroke(
                    rect,
                    10.0,
                    egui::Stroke::new(2.0, color),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "PNGまたはJPEG画像を\nここにドラッグ＆ドロップ",
                    egui::FontId::proportional(22.0),
                    egui::Color32::LIGHT_GRAY,
                );
            }
        });
    }
}

fn show_exif(ui: &mut egui::Ui, metadata: &ImageMetadata) {
    ui.heading(format!("Exif（{}件）", metadata.exif.len()));
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("exif_scroll")
        .show(ui, |ui| {
            if metadata.exif.is_empty() {
                ui.weak("Exif情報はありません。");
            }
            egui::Grid::new("exif_grid")
                .num_columns(2)
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for field in &metadata.exif {
                        ui.label(&field.name).on_hover_text(&field.group);
                        ui.label(&field.value);
                        ui.end_row();
                    }
                });
        });
}

fn show_xmp(ui: &mut egui::Ui, metadata: &ImageMetadata) {
    ui.heading("XMP");
    ui.separator();
    egui::ScrollArea::both()
        .id_salt("xmp_scroll")
        .show(ui, |ui| {
            if let Some(xmp) = &metadata.xmp {
                let mut text = xmp.clone();
                ui.add(
                    egui::TextEdit::multiline(&mut text)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .interactive(false),
                );
            } else {
                ui.weak("XMP情報はありません。");
            }
        });
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg"
            )
        })
}
