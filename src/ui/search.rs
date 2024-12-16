use super::recipe::RecipeWindow;
use crate::database::{
    self,
    models::{IngredientHandle, RecipeHandle, RecipeId},
};
use diesel::ExpressionMethods as _;
use diesel::JoinOnDsl as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use eframe::egui;
use std::collections::HashMap;
use std::hash::Hash;

pub struct SearchWidget<'a, SearchFn, ValueT> {
    buf: &'a mut String,
    value: &'a mut Option<ValueT>,
    search_fn: SearchFn,
    pop_up_id: egui::Id,
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

pub enum SearchParam {
    IngredientEqual(IngredientHandle),
}

pub struct SearchWindow {
    id: u64,
    query: Vec<SearchParam>,
    results: Vec<RecipeHandle>,
}

impl SearchWindow {
    pub fn new(id: u64, query: Vec<SearchParam>) -> Self {
        Self {
            id,
            query,
            results: vec![],
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
    ) -> bool {
        use database::schema::{ingredient_usages, ingredients, recipes};

        self.results = if self.query.is_empty() {
            vec![]
        } else {
            let SearchParam::IngredientEqual(i) = &self.query[0];
            recipes::table
                .inner_join(
                    ingredient_usages::table.on(ingredient_usages::recipe_id.eq(recipes::id)),
                )
                .inner_join(
                    ingredients::table.on(ingredient_usages::ingredient_id.eq(ingredients::id)),
                )
                .filter(ingredients::id.eq(i.id))
                .select(RecipeHandle::as_select())
                .load(conn)
                .unwrap()
        };

        let mut open = true;
        egui::Window::new("Search")
            .id(egui::Id::new(("search window", self.id)))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for param in &self.query {
                        match param {
                            SearchParam::IngredientEqual(i) => {
                                ui.label(format!("ingredient == {}", &i.name));
                            }
                        }
                    }

                    for recipe in &self.results {
                        let mut shown = recipe_windows.contains_key(&recipe.id);
                        ui.toggle_value(&mut shown, recipe.name.clone());

                        if shown && !recipe_windows.contains_key(&recipe.id) {
                            recipe_windows
                                .insert(recipe.id, RecipeWindow::new(conn, recipe.id, false));
                        } else if !shown {
                            recipe_windows.remove(&recipe.id);
                        }
                    }
                });
            });
        !open
    }
}
