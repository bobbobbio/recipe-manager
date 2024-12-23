use crate::database;
use crate::import;

#[derive(Default)]
pub enum ImportWindow {
    #[default]
    Ready,
    ImportingRecipes {
        importer: crate::import::RecipeImporter,
        log: String,
    },
    ImportingCalendar {
        importer: crate::import::CalendarImporter,
        log: String,
    },
    Failed {
        error: crate::Error,
    },
    Success {
        num_imported: usize,
        log: String,
    },
}

pub enum UpdateEvent {
    Closed,
    Imported,
}

impl ImportWindow {
    pub fn update(
        &mut self,
        conn: &mut database::Connection,
        ctx: &egui::Context,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        let mut open = true;
        egui::Window::new("Import Data from Previous Version")
            .open(&mut open)
            .show(ctx, |ui| {
                let next = match self {
                    Self::Ready => Self::update_ready(conn, ui),
                    Self::ImportingRecipes { log, importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_importing(conn, log, importer, &mut events, ui)
                    }
                    Self::ImportingCalendar { log, importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_importing(conn, log, importer, &mut events, ui)
                    }
                    Self::Failed { error } => Self::update_failed(error, ui),
                    Self::Success { num_imported, log } => {
                        Self::update_success(*num_imported, log, ui)
                    }
                };
                if let Some(next) = next {
                    *self = next;
                }
            });
        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }

    fn update_ready(conn: &mut database::Connection, ui: &mut egui::Ui) -> Option<Self> {
        ui.label("This dialog lets you import data from older versions of Recipe Manager.");
        ui.horizontal(|ui| {
            if ui.button("Import Recipes").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("recipebook", &["recipebook"])
                    .set_directory("/")
                    .pick_file()
                {
                    return Some(match import::RecipeImporter::new(conn, file) {
                        Ok(importer) => Self::ImportingRecipes {
                            importer,
                            log: String::new(),
                        },
                        Err(error) => Self::Failed { error },
                    });
                }
            }
            if ui.button("Import Calendar").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("recipecalendar", &["recipecalendar"])
                    .set_directory("/")
                    .pick_file()
                {
                    return Some(match import::CalendarImporter::new(file) {
                        Ok(importer) => Self::ImportingCalendar {
                            importer,
                            log: String::new(),
                        },
                        Err(error) => Self::Failed { error },
                    });
                }
            }
            None
        })
        .inner
    }

    fn update_importing(
        conn: &mut database::Connection,
        log: &mut String,
        importer: &mut impl import::Importer,
        events: &mut Vec<UpdateEvent>,
        ui: &mut egui::Ui,
    ) -> Option<Self> {
        ui.label("importing data..");
        ui.add(egui::widgets::ProgressBar::new(importer.percent_done()));

        if !importer.done() {
            if let Err(error) = importer.import_one(conn, log) {
                return Some(Self::Failed { error });
            }
        } else {
            events.push(UpdateEvent::Imported);
            return Some(Self::Success {
                num_imported: importer.num_imported(),
                log: std::mem::take(log),
            });
        }

        None
    }

    fn update_failed(error: &crate::Error, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!("import failed with error: {error}"));
        ui.button("okay").clicked().then_some(Self::Ready)
    }

    fn update_success(num_imported: usize, log: &str, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!("import succeeded. {num_imported} items imported."));
        if !log.is_empty() {
            let scroll_height = ui.available_height() - 35.0;
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .max_height(scroll_height)
                .show(ui, |ui| {
                    ui.label(log);
                });
        }
        ui.separator();
        ui.button("okay").clicked().then_some(Self::Ready)
    }
}
