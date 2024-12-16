// Copyright 2023 Remi Bernotavicius

mod calendar;
mod category_list;
mod import;
mod ingredient_list;
mod query;
mod recipe;
mod recipe_list;
mod search;

use crate::database;
use crate::database::models::{IngredientHandle, RecipeCategoryId, RecipeId};
use calendar::CalendarWindow;
use category_list::CategoryListWindow;
use eframe::egui;
use import::ImportWindow;
use ingredient_list::IngredientListWindow;
use recipe::RecipeWindow;
use recipe_list::RecipeListWindow;
use search::SearchResultsWindow;
use std::collections::HashMap;
use std::mem;

pub fn new_error_toast(msg: impl Into<egui::WidgetText>) -> egui_toast::Toast {
    egui_toast::Toast {
        text: msg.into(),
        kind: egui_toast::ToastKind::Error,
        options: egui_toast::ToastOptions::default()
            .duration_in_seconds(3.0)
            .show_progress(false)
            .show_icon(true),
        ..Default::default()
    }
}

pub struct RecipeManager {
    category_list: CategoryListWindow,
    conn: database::Connection,
    toasts: egui_toast::Toasts,
    import_window: Option<ImportWindow>,
    recipe_lists: HashMap<RecipeCategoryId, RecipeListWindow>,
    recipes: HashMap<RecipeId, RecipeWindow>,
    ingredient_list_window: Option<IngredientListWindow>,
    calendar_window: Option<CalendarWindow>,
    search_result_windows: Vec<SearchResultsWindow>,
    next_search_window_id: u64,
}

impl RecipeManager {
    pub fn new(mut conn: database::Connection) -> Self {
        Self {
            category_list: CategoryListWindow::new(&mut conn),
            conn,
            import_window: None,
            recipe_lists: Default::default(),
            recipes: Default::default(),
            ingredient_list_window: None,
            calendar_window: None,
            search_result_windows: Default::default(),
            next_search_window_id: 0,
            toasts: egui_toast::Toasts::new()
                .anchor(egui::Align2::LEFT_BOTTOM, (10.0, 10.0))
                .direction(egui::Direction::BottomUp),
        }
    }

    fn ingredient_search(
        conn: &mut database::Connection,
        search_result_windows: &mut Vec<SearchResultsWindow>,
        next_search_window_id: &mut u64,
        ingredient: IngredientHandle,
    ) {
        let results = query::search_recipes_by_ingredient(conn, ingredient.id);
        let query = format!("Recipes using \"{}\"", &ingredient.name);
        search_result_windows.push(SearchResultsWindow::new(
            *next_search_window_id,
            query,
            results,
        ));
        *next_search_window_id += 1;
    }

    fn update_category_list_window(&mut self, ctx: &egui::Context) {
        self.category_list.update(
            ctx,
            &mut self.conn,
            &mut self.toasts,
            &mut self.recipe_lists,
        );
    }

    fn update_recipe_list_windows(&mut self, ctx: &egui::Context) {
        for (id, mut list) in mem::take(&mut self.recipe_lists) {
            let closed = list.update(ctx, &mut self.conn, &mut self.recipes);

            if !closed {
                self.recipe_lists.insert(id, list);
            }
        }
    }

    fn update_recipes(&mut self, ctx: &egui::Context) {
        for (id, mut recipe) in mem::take(&mut self.recipes) {
            let mut closed = false;
            let events = recipe.update(ctx, &mut self.conn, &mut self.toasts);
            for e in events {
                match e {
                    recipe::UpdateEvent::Closed => closed = true,
                    recipe::UpdateEvent::Renamed(recipe) => {
                        if let Some(list) = self.recipe_lists.get_mut(&recipe.category) {
                            list.recipe_name_changed(recipe.id, recipe.name);
                        }
                    }
                    recipe::UpdateEvent::Scheduled => {
                        if let Some(c) = self.calendar_window.as_mut() {
                            c.refresh(&mut self.conn);
                        }
                    }
                    recipe::UpdateEvent::CategoryChanged => {
                        for r in self.recipe_lists.values_mut() {
                            r.recipe_category_changed(&mut self.conn);
                        }
                    }
                }
            }

            if !closed {
                self.recipes.insert(id, recipe);
            }
        }
    }

    fn update_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import").clicked() {
                        if self.import_window.is_none() {
                            self.import_window = Some(ImportWindow::default());
                        }
                        ui.close_menu();
                    }
                    if ui.button("Ingredients").clicked() {
                        if self.ingredient_list_window.is_none() {
                            self.ingredient_list_window =
                                Some(IngredientListWindow::new(&mut self.conn, false));
                        }
                        ui.close_menu();
                    }
                    if ui.button("Calendar").clicked() {
                        if self.calendar_window.is_none() {
                            self.calendar_window = Some(CalendarWindow::new(&mut self.conn));
                        }
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn update_import_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.import_window {
            if window.update(&mut self.conn, ctx) {
                self.import_window = None;
            }
        }
    }

    fn update_ingredient_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.ingredient_list_window {
            let search_for_ingredient = |conn: &mut database::Connection, ingredient| {
                Self::ingredient_search(
                    conn,
                    &mut self.search_result_windows,
                    &mut self.next_search_window_id,
                    ingredient,
                )
            };
            let events =
                window.update(&mut self.conn, &mut self.toasts, search_for_ingredient, ctx);
            for e in events {
                match e {
                    ingredient_list::UpdateEvent::Closed => self.ingredient_list_window = None,
                    ingredient_list::UpdateEvent::IngredientEdited(ingredient) => {
                        for r in self.recipes.values_mut() {
                            r.ingredient_edited(&mut self.conn, ingredient.clone());
                        }
                    }
                }
            }
        }
    }

    fn update_calendar_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.calendar_window {
            let events = window.update(ctx, &mut self.conn, &mut self.toasts);
            for e in events {
                match e {
                    calendar::UpdateEvent::Closed => {
                        self.calendar_window = None;
                    }
                    calendar::UpdateEvent::RecipeScheduled { week } => {
                        for recipe in self.recipes.values_mut() {
                            recipe.recipe_scheduled(&mut self.conn, week);
                        }
                    }
                }
            }
        }
    }

    fn update_search_result_windows(&mut self, ctx: &egui::Context) {
        for mut sw in mem::take(&mut self.search_result_windows) {
            if !sw.update(ctx, &mut self.conn, &mut self.recipes) {
                self.search_result_windows.push(sw);
            }
        }
    }
}

impl eframe::App for RecipeManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_menu(ctx);
        self.update_import_window(ctx);
        self.update_ingredient_window(ctx);
        self.update_category_list_window(ctx);
        self.update_recipe_list_windows(ctx);
        self.update_recipes(ctx);
        self.update_calendar_window(ctx);
        self.update_search_result_windows(ctx);
        self.toasts.show(ctx);
    }
}
