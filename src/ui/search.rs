use super::{new_error_toast, query, recipe::RecipeWindow};
use crate::database::{
    self,
    models::{Ingredient, IngredientHandle, RecipeHandle, RecipeId},
};
use eframe::egui;
use std::collections::HashMap;
use std::hash::Hash;

pub struct SearchWidget<'a, SearchFn, ValueT> {
    buf: &'a mut String,
    value: &'a mut Option<ValueT>,
    search_fn: SearchFn,
    pop_up_id: egui::Id,
    hint_text: Option<egui::WidgetText>,
    desired_width: Option<f32>,
}

impl<'a, SearchFn, ValueT> SearchWidget<'a, SearchFn, ValueT>
where
    SearchFn: FnOnce(&str) -> Vec<(ValueT, String)>,
{
    pub fn new(
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
            hint_text: None,
            desired_width: None,
        }
    }

    pub fn hint_text(mut self, hint_text: impl Into<egui::WidgetText>) -> Self {
        self.hint_text = Some(hint_text.into());
        self
    }

    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
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
            hint_text,
            desired_width,
        } = self;

        let mut edit = egui::TextEdit::singleline(buf);
        if let Some(hint_text) = hint_text {
            edit = edit.hint_text(hint_text);
        }
        if let Some(desired_width) = desired_width {
            edit = edit.desired_width(desired_width);
        }
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

pub struct SearchResultsWindow {
    id: u64,
    query: String,
    results: Vec<RecipeHandle>,
}

impl SearchResultsWindow {
    pub fn new(id: u64, query: String, results: Vec<RecipeHandle>) -> Self {
        Self { id, query, results }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
    ) -> bool {
        let mut open = true;
        egui::Window::new("Search Results")
            .id(egui::Id::new(("search window", self.id)))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(&self.query);

                    let scroll_height = ui.available_height() - 45.0;
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .max_height(scroll_height)
                        .show(ui, |ui| {
                            for recipe in &self.results {
                                let mut shown = recipe_windows.contains_key(&recipe.id);
                                ui.toggle_value(&mut shown, recipe.name.clone());

                                if shown && !recipe_windows.contains_key(&recipe.id) {
                                    recipe_windows.insert(
                                        recipe.id,
                                        RecipeWindow::new(conn, recipe.id, false),
                                    );
                                } else if !shown {
                                    recipe_windows.remove(&recipe.id);
                                }
                            }
                            if self.results.is_empty() {
                                ui.label("Nothing found");
                            }
                        });
                });
            });
        !open
    }
}

pub struct RecipeSearchWindow {
    to_search: Vec<IngredientHandle>,

    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,
}

impl RecipeSearchWindow {
    pub fn new() -> Self {
        Self {
            to_search: vec![],
            new_ingredient_name: String::new(),
            new_ingredient: None,
            cached_ingredient_search: None,
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        mut search_for_ingredients: impl FnMut(&mut database::Connection, Vec<IngredientHandle>),
    ) -> bool {
        let mut open = true;
        egui::Window::new("Recipe Search")
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("Recipe Search").show(ui, |ui| {
                    for ingredient in std::mem::take(&mut self.to_search) {
                        ui.label(&ingredient.name);
                        if !ui.button("Remove").clicked() {
                            self.to_search.push(ingredient);
                        }
                        ui.end_row();
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(
                        SearchWidget::new(
                            "recipe search ingredient name",
                            &mut self.new_ingredient_name,
                            &mut self.new_ingredient,
                            |query| {
                                query::search_ingredients(
                                    conn,
                                    &mut self.cached_ingredient_search,
                                    query,
                                )
                            },
                        )
                        .hint_text("search for ingredient"),
                    );
                    if ui.button("Add").clicked() {
                        if let Some(ingredient) = &self.new_ingredient {
                            self.to_search.push(ingredient.to_handle());
                            self.new_ingredient_name = "".into();
                            self.new_ingredient = None;
                        } else {
                            toasts.add(new_error_toast("Couldn't find ingredient"));
                        }
                    }
                });
                if !self.to_search.is_empty() && ui.button("Search").clicked() {
                    search_for_ingredients(conn, self.to_search.clone());
                }
            });

        !open
    }
}
