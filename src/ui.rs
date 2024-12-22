// Copyright 2023 Remi Bernotavicius

mod about;
mod calendar;
mod category_list;
mod generate_rtf;
mod import;
mod ingredient_calories;
mod ingredient_list;
mod query;
mod recipe;
mod recipe_list;
mod search;
mod unit_conversion;

use crate::database;
use crate::database::models::{IngredientHandle, IngredientId, RecipeCategoryId, RecipeId};
use about::AboutWindow;
use calendar::CalendarWindow;
use category_list::CategoryListWindow;
use eframe::egui;
use import::ImportWindow;
use ingredient_calories::IngredientCaloriesWindow;
use ingredient_list::IngredientListWindow;
use recipe::RecipeWindow;
use recipe_list::RecipeListWindow;
use search::{IngredientSearchControl, RecipeSearchWindow, SearchResultsWindow};
use std::collections::HashMap;
use std::mem;

pub fn new_error_toast(msg: impl Into<egui::WidgetText>) -> egui_toast::Toast {
    egui_toast::Toast {
        text: msg.into(),
        kind: egui_toast::ToastKind::Error,
        options: egui_toast::ToastOptions::default()
            .duration_in_seconds(10.0)
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
    next_search_results_window_id: u64,
    recipe_search_window: Option<RecipeSearchWindow>,
    ingredient_calories_windows: HashMap<IngredientId, IngredientCaloriesWindow>,
    about_window: Option<AboutWindow>,
}

impl RecipeManager {
    pub fn new(mut conn: database::Connection) -> Self {
        Self {
            category_list: CategoryListWindow::new(&mut conn, false),
            conn,
            import_window: None,
            recipe_lists: Default::default(),
            recipes: Default::default(),
            ingredient_list_window: None,
            calendar_window: None,
            search_result_windows: Default::default(),
            next_search_results_window_id: 0,
            recipe_search_window: None,
            ingredient_calories_windows: Default::default(),
            about_window: None,
            toasts: egui_toast::Toasts::new()
                .anchor(egui::Align2::LEFT_BOTTOM, (10.0, 10.0))
                .direction(egui::Direction::BottomUp),
        }
    }

    fn ingredient_search(
        conn: &mut database::Connection,
        search_result_windows: &mut Vec<SearchResultsWindow>,
        next_search_results_window_id: &mut u64,
        control: IngredientSearchControl,
        ingredients: Vec<IngredientHandle>,
    ) {
        let ingredient_ids: Vec<_> = ingredients.iter().map(|i| i.id).collect();
        let qualifier;
        let results = match control {
            IngredientSearchControl::All => {
                qualifier = "all".into();
                query::search_recipes_including_all_ingredient(conn, ingredient_ids)
            }
            IngredientSearchControl::Any => {
                qualifier = "any".into();
                query::search_recipes_including_any_ingredient(conn, ingredient_ids)
            }
            IngredientSearchControl::AtLeast(at_least) => {
                qualifier = format!("at least {at_least}");
                query::search_recipes_including_at_least_ingredients(conn, ingredient_ids, at_least)
            }
        };
        let query = if ingredients.len() == 1 {
            format!("Recipes using \"{}\"", &ingredients[0].name)
        } else {
            let mut query = format!("Recipes using {qualifier} of \"{}\"", &ingredients[0].name);
            for i in &ingredients[1..] {
                query += &format!(", \"{}\"", &i.name);
            }
            query
        };

        search_result_windows.push(SearchResultsWindow::new(
            *next_search_results_window_id,
            query,
            results,
        ));
        *next_search_results_window_id += 1;
    }

    fn update_category_list_window(&mut self, ctx: &egui::Context) {
        self.category_list.update(
            ctx,
            &mut self.conn,
            &mut self.toasts,
            &mut self.recipe_lists,
        );
    }

    fn update_about_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.about_window {
            if window.update(ctx) {
                self.about_window = None;
            }
        }
    }

    fn update_recipe_list_windows(&mut self, ctx: &egui::Context) {
        for (id, mut list) in mem::take(&mut self.recipe_lists) {
            let mut closed = false;
            let events = list.update(ctx, &mut self.conn, &mut self.recipes);
            for event in events {
                match event {
                    recipe_list::UpdateEvent::Closed => closed = true,
                    recipe_list::UpdateEvent::RecipeDeleted(id) => {
                        for s in &mut self.search_result_windows {
                            s.recipe_deleted(id);
                        }
                    }
                }
            }

            if !closed {
                self.recipe_lists.insert(id, list);
            }
        }
    }

    fn update_recipes(&mut self, ctx: &egui::Context) {
        let mut recipe_scheduled = vec![];
        for (id, mut recipe) in mem::take(&mut self.recipes) {
            let mut closed = false;
            let events = recipe.update(
                ctx,
                &mut self.conn,
                &mut self.toasts,
                &mut self.ingredient_calories_windows,
            );
            for e in events {
                match e {
                    recipe::UpdateEvent::Closed => closed = true,
                    recipe::UpdateEvent::Renamed(recipe) => {
                        if let Some(list) = self.recipe_lists.get_mut(&recipe.category) {
                            list.recipe_name_changed(recipe.id, recipe.name);
                        }
                    }
                    recipe::UpdateEvent::Scheduled(week) => {
                        recipe_scheduled.push(week);
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

        for week in recipe_scheduled {
            if let Some(c) = self.calendar_window.as_mut() {
                c.recipe_scheduled(&mut self.conn);
            }
            for recipe in self.recipes.values_mut() {
                recipe.recipe_scheduled(&mut self.conn, week);
            }
        }
    }

    fn update_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Window", |ui| {
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
                    if ui.button("Recipe Search").clicked() {
                        if self.recipe_search_window.is_none() {
                            self.recipe_search_window = Some(RecipeSearchWindow::new());
                        }
                        ui.close_menu();
                    }
                    if ui.button("About").clicked() {
                        if self.about_window.is_none() {
                            self.about_window = Some(AboutWindow::new());
                        }
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn update_import_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.import_window {
            let events = window.update(&mut self.conn, ctx);
            for e in events {
                match e {
                    import::UpdateEvent::Closed => {
                        self.import_window = None;
                    }
                    import::UpdateEvent::Imported => {
                        self.category_list.recipes_imported(&mut self.conn);
                        if let Some(c) = &mut self.calendar_window {
                            c.calendar_imported(&mut self.conn);
                        }
                    }
                }
            }
        }
    }

    fn update_ingredient_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.ingredient_list_window {
            let search_for_ingredient = |conn: &mut database::Connection, ingredients| {
                Self::ingredient_search(
                    conn,
                    &mut self.search_result_windows,
                    &mut self.next_search_results_window_id,
                    IngredientSearchControl::All,
                    ingredients,
                )
            };
            let events = window.update(
                &mut self.conn,
                &mut self.toasts,
                &mut self.ingredient_calories_windows,
                search_for_ingredient,
                ctx,
            );
            for e in events {
                match e {
                    ingredient_list::UpdateEvent::Closed => self.ingredient_list_window = None,
                    ingredient_list::UpdateEvent::IngredientEdited => {
                        for r in self.recipes.values_mut() {
                            r.ingredient_edited(&mut self.conn);
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

    fn update_recipe_search_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.recipe_search_window {
            let search_by_ingredients = |conn: &mut database::Connection, control, ingredients| {
                Self::ingredient_search(
                    conn,
                    &mut self.search_result_windows,
                    &mut self.next_search_results_window_id,
                    control,
                    ingredients,
                )
            };
            if window.update(ctx, &mut self.conn, &mut self.toasts, search_by_ingredients) {
                self.recipe_search_window = None;
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

    fn update_ingredient_calories_windows(&mut self, ctx: &egui::Context) {
        for (id, mut ingredient_calories) in mem::take(&mut self.ingredient_calories_windows) {
            let mut closed = false;
            let events = ingredient_calories.update(ctx, &mut self.conn);
            for event in events {
                match event {
                    ingredient_calories::UpdateEvent::Closed => closed = true,
                    ingredient_calories::UpdateEvent::IngredientEdited => {
                        for r in self.recipes.values_mut() {
                            r.ingredient_edited(&mut self.conn);
                        }
                    }
                }
            }
            if !closed {
                self.ingredient_calories_windows
                    .insert(id, ingredient_calories);
            }
        }
    }
}

impl eframe::App for RecipeManager {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        if visuals.dark_mode {
            egui::Color32::from_rgba_unmultiplied(20, 20, 20, 255).to_normalized_gamma_f32()
        } else {
            egui::Color32::from_rgba_unmultiplied(252, 252, 252, 255).to_normalized_gamma_f32()
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui_extras::install_image_loaders(ctx);

        self.update_menu(ctx);
        self.update_import_window(ctx);
        self.update_ingredient_window(ctx);
        self.update_category_list_window(ctx);
        self.update_recipe_list_windows(ctx);
        self.update_recipes(ctx);
        self.update_calendar_window(ctx);
        self.update_search_result_windows(ctx);
        self.update_recipe_search_window(ctx);
        self.update_ingredient_calories_windows(ctx);
        self.update_about_window(ctx);
        self.toasts.show(ctx);
    }
}
