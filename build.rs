// Copyright 2023 Remi Bernotavicius

use std::process::Command;
use std::{env, fs};

fn main() {
    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("images/appicon.ico")
        .compile()
        .unwrap();

    // On windows, the patch part has problems for some reason
    if cfg!(windows) {
        return;
    }

    println!("cargo:rerun-if-changed=migrations/");
    println!("cargo:rerun-if-changed=schema_fix.patch");

    let out_dir = env::temp_dir();
    let database_path = out_dir.join("database.sqlite");
    if database_path.exists() {
        fs::remove_file(&database_path).unwrap();
    }

    let status = Command::new("diesel")
        .args([
            "migration",
            "run",
            "--database-url",
            database_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    fs::remove_file(&database_path).unwrap();
}
