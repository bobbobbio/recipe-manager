// Copyright 2023 Remi Bernotavicius

use std::path::PathBuf;

mod database;
mod import;
mod ui;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T> = std::result::Result<T, Error>;

/// This is where the database and other user-data lives on-disk. On Linux it should be like:
/// `~/.local/share/recipe_manager/`
fn data_path() -> Result<PathBuf> {
    let dirs = directories::BaseDirs::new().expect("failed to get user home directory");
    let path = dirs.data_dir().join("recipe-manager");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

fn run(conn: database::Connection) -> Result<()> {
    let native_options = eframe::NativeOptions {
        window_builder: Some(Box::new(|mut b: egui::viewport::ViewportBuilder| {
            b.maximized = Some(true);
            b.transparent = Some(false);
            b
        })),
        ..Default::default()
    };
    eframe::run_native(
        "Recipe Manager",
        native_options,
        Box::new(|_cc| Ok(Box::new(ui::RecipeManager::new(conn)))),
    )
    .unwrap();

    Ok(())
}

fn main() -> Result<()> {
    let conn = database::establish_connection(data_path()?.join("data.sqlite"))?;
    run(conn)?;
    Ok(())
}
