use crate::database;
use crate::import;
use eframe::egui;

#[derive(Default)]
pub enum ImportWindow {
    #[default]
    Ready,
    ImportingRecipes {
        importer: crate::import::RecipeImporter,
    },
    ImportingCalendar {
        importer: crate::import::CalendarImporter,
    },
    Failed {
        error: crate::Error,
    },
    Success {
        num_imported: usize,
    },
}

impl ImportWindow {
    pub fn update(&mut self, conn: &mut database::Connection, ctx: &egui::Context) -> bool {
        let mut open = true;
        egui::Window::new("Import data")
            .open(&mut open)
            .show(ctx, |ui| {
                let next = match self {
                    Self::Ready => Self::update_ready(conn, ui),
                    Self::ImportingRecipes { importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_importing(conn, importer, ui)
                    }
                    Self::ImportingCalendar { importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_importing(conn, importer, ui)
                    }
                    Self::Failed { error } => Self::update_failed(error, ui),
                    Self::Success { num_imported } => Self::update_success(*num_imported, ui),
                };
                if let Some(next) = next {
                    *self = next;
                }
            });
        !open
    }

    fn update_ready(conn: &mut database::Connection, ui: &mut egui::Ui) -> Option<Self> {
        if ui.button("import recipes").clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter("recipebook", &["recipebook"])
                .set_directory("/")
                .pick_file()
            {
                return Some(match import::RecipeImporter::new(conn, file) {
                    Ok(importer) => Self::ImportingRecipes { importer },
                    Err(error) => Self::Failed { error },
                });
            }
        }
        if ui.button("import calendar").clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter("recipecalendar", &["recipecalendar"])
                .set_directory("/")
                .pick_file()
            {
                return Some(match import::CalendarImporter::new(file) {
                    Ok(importer) => Self::ImportingCalendar { importer },
                    Err(error) => Self::Failed { error },
                });
            }
        }
        None
    }

    fn update_importing(
        conn: &mut database::Connection,
        importer: &mut impl import::Importer,
        ui: &mut egui::Ui,
    ) -> Option<Self> {
        ui.label("importing data..");
        ui.add(egui::widgets::ProgressBar::new(importer.percent_done()));

        if !importer.done() {
            if let Err(error) = importer.import_one(conn) {
                return Some(Self::Failed { error });
            }
        } else {
            return Some(Self::Success {
                num_imported: importer.num_imported(),
            });
        }

        None
    }

    fn update_failed(error: &crate::Error, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!("import failed with error: {error}"));
        ui.button("okay").clicked().then_some(Self::Ready)
    }

    fn update_success(num_imported: usize, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!(
            "import succeeded. {num_imported} recipes imported."
        ));
        ui.button("okay").clicked().then_some(Self::Ready)
    }
}
