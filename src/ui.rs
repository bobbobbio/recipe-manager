// Copyright 2023 Remi Bernotavicius

mod calendar;
mod category_list;
mod import;
mod ingredient_list;
mod query;
mod recipe;
mod recipe_list;

use crate::database;
use crate::database::models::{RecipeCategoryId, RecipeId};
use calendar::CalendarWindow;
use category_list::CategoryListWindow;
use eframe::egui;
use import::ImportWindow;
use ingredient_list::IngredientListWindow;
use recipe::RecipeWindow;
use recipe_list::RecipeListWindow;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem;

struct SearchWidget<'a, SearchFn, ValueT> {
    buf: &'a mut String,
    value: &'a mut Option<ValueT>,
    search_fn: SearchFn,
    pop_up_id: egui::Id,
}

impl<'a, SearchFn, ValueT> SearchWidget<'a, SearchFn, ValueT>
where
    SearchFn: FnOnce(&str) -> Vec<(ValueT, String)>,
{
    fn new(
        id_source: impl Hash,
        buf: &'a mut String,
        value: &'a mut Option<ValueT>,
        search_fn: SearchFn,
    ) -> Self {
        Self {
            buf,
            value,
            search_fn,
            pop_up_id: egui::Id::new(id_source),
        }
    }
}

impl<'a, SearchFn, ValueT> egui::Widget for SearchWidget<'a, SearchFn, ValueT>
where
    SearchFn: FnOnce(&str) -> Vec<(ValueT, String)>,
    ValueT: Clone,
{
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            pop_up_id,
            buf,
            value,
            search_fn,
        } = self;

        let edit = egui::TextEdit::singleline(buf);
        let edit_output = edit.show(ui);
        let mut r = edit_output.response;
        if r.gained_focus() {
            ui.memory_mut(|m| m.open_popup(pop_up_id));
        }

        let mut changed = false;
        egui::popup_below_widget(
            ui,
            pop_up_id,
            &r,
            egui::PopupCloseBehavior::CloseOnClick,
            |ui| {
                egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .show(ui, |ui| {
                        let mut matches_valid = false;
                        for (text_id, text) in search_fn(buf) {
                            if buf == &text {
                                matches_valid = true;
                                if value.is_none() {
                                    *value = Some(text_id.clone());
                                }
                            }

                            if ui.selectable_label(false, &text).clicked() {
                                *value = Some(text_id);
                                *buf = text;
                                changed = true;
                                ui.memory_mut(|m| m.close_popup());
                                matches_valid = true;
                            }
                        }
                        if !matches_valid {
                            *value = None;
                        }
                    });
            },
        );

        if changed {
            r.mark_changed();
        }

        r
    }
}

pub struct RecipeManager {
    category_list: CategoryListWindow,
    conn: database::Connection,
    import_window: Option<ImportWindow>,
    recipe_lists: HashMap<RecipeCategoryId, RecipeListWindow>,
    recipes: HashMap<RecipeId, RecipeWindow>,
    ingredient_list_window: Option<IngredientListWindow>,
    calendar_window: Option<CalendarWindow>,
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
        }
    }

    fn update_category_list_window(&mut self, ctx: &egui::Context) {
        self.category_list
            .update(ctx, &mut self.conn, &mut self.recipe_lists);
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
            let events = recipe.update(ctx, &mut self.conn);
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
                    if ui.button("Import").clicked() && self.import_window.is_none() {
                        self.import_window = Some(ImportWindow::default());
                        ui.close_menu();
                    }
                    if ui.button("Ingredients").clicked() && self.ingredient_list_window.is_none() {
                        self.ingredient_list_window =
                            Some(IngredientListWindow::new(&mut self.conn));
                        ui.close_menu();
                    }
                    if ui.button("Calendar").clicked() && self.calendar_window.is_none() {
                        self.calendar_window = Some(CalendarWindow::new(&mut self.conn));
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
            if window.update(&mut self.conn, ctx) {
                self.ingredient_list_window = None;
            }
        }
    }

    fn update_calendar_window(&mut self, ctx: &egui::Context) {
        if let Some(window) = &mut self.calendar_window {
            if window.update(&mut self.conn, ctx) {
                self.calendar_window = None;
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
    }
}
