use super::{new_error_toast, query, recipe::RecipeWindow, PressedEnterExt as _};
use crate::database::{
    self,
    models::{Ingredient, IngredientHandle, IngredientId, RecipeHandle, RecipeId},
};
use derive_more::Display;
use std::collections::HashMap;
use std::hash::Hash;
use strum::{EnumIter, IntoEnumIterator as _};

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
        let under_text = r.rect.bottom();
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
                let results = search_fn(buf);

                let style = ui.style();
                let button_height = (egui::TextStyle::Button.resolve(&style).size
                    + style.spacing.button_padding.y as f32 * 2.0)
                    .max(style.spacing.interact_size.y);
                let spacing = style.spacing.item_spacing.y;
                let height_for_size =
                    |l: usize| button_height * l as f32 + spacing * l.saturating_sub(1) as f32;

                let remaining_height = ui.ctx().screen_rect().height() - under_text - 12.0;
                let contents_height = height_for_size(results.len());
                ui.set_height(
                    contents_height
                        .min(height_for_size(19))
                        .min(remaining_height),
                );

                egui::ScrollArea::vertical()
                    .max_height(f32::INFINITY)
                    .show(ui, |ui| {
                        let mut matches_valid = false;
                        for (text_id, text) in results {
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
            if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                let ccursor = egui::text::CCursor::new(buf.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(ccursor)));
                state.store(ui.ctx(), r.id);
            }

            r.request_focus();
            ui.memory_mut(|m| m.open_popup(pop_up_id));
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

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
        ui: &mut egui::Ui,
    ) {
        if self.results.is_empty() {
            ui.add(egui::Label::new(&self.query).wrap());
            ui.label("Nothing found");
            return;
        }

        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt(("search results table", self.id))
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.add(egui::Label::new(&self.query).wrap());
                });
            })
            .body(|mut body| {
                for recipe in &self.results {
                    body.row(20.0, |mut row| {
                        let mut shown = recipe_windows.contains_key(&recipe.id);
                        row.col(|ui| {
                            ui.toggle_value(&mut shown, recipe.name.clone());
                        });

                        if shown && !recipe_windows.contains_key(&recipe.id) {
                            recipe_windows
                                .insert(recipe.id, RecipeWindow::new(conn, recipe.id, false));
                        } else if !shown {
                            recipe_windows.remove(&recipe.id);
                        }
                    });
                }
            });
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
                self.update_table(conn, recipe_windows, ui);
            });
        !open
    }

    pub fn recipe_deleted(&mut self, recipe_id: RecipeId) {
        self.results.retain(|handle| handle.id != recipe_id);
    }
}

#[derive(Copy, Clone, Display, PartialEq, Eq)]
pub enum IngredientSearchControl {
    #[display("all")]
    All,
    #[display("any")]
    Any,
    #[display("at least")]
    AtLeast(usize),
}

impl IngredientSearchControl {
    fn iter() -> [Self; 3] {
        [Self::All, Self::Any, Self::AtLeast(2)]
    }
}

struct RecipeSearchByIngredient {
    to_search: Vec<IngredientHandle>,

    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,
    control: IngredientSearchControl,
}

impl RecipeSearchByIngredient {
    fn new() -> Self {
        Self {
            to_search: vec![],
            new_ingredient_name: String::new(),
            new_ingredient: None,
            cached_ingredient_search: None,
            control: IngredientSearchControl::All,
        }
    }

    fn update_table(&mut self, ui: &mut egui::Ui) {
        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt("")
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .column(egui_extras::Column::exact(60.0))
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .body(|mut body| {
                for ingredient in std::mem::take(&mut self.to_search) {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.label(&ingredient.name);
                        });
                        row.col(|ui| {
                            if !ui.button("Remove").clicked() {
                                self.to_search.push(ingredient);
                            }
                        });
                    });
                }
            });
    }

    fn update_add_ingredient(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
    ) {
        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder())
            .size(egui_extras::Size::exact(40.0))
            .horizontal(|mut strip| {
                let mut added = false;
                strip.cell(|ui| {
                    added |= ui
                        .add(
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
                            .hint_text("search for ingredient")
                            .desired_width(f32::INFINITY),
                        )
                        .pressed_enter();
                });
                let e = !self.new_ingredient_name.is_empty();
                strip.cell(|ui| {
                    added |= ui.add_enabled(e, egui::Button::new("Add")).clicked();
                });

                if added && e {
                    if let Some(ingredient) = &self.new_ingredient {
                        if self.to_search.iter().any(|i| i.id == ingredient.id) {
                            toasts.add(new_error_toast("Ingredient already in search"));
                        } else {
                            self.to_search.push(ingredient.to_handle());
                            self.new_ingredient_name = "".into();
                            self.new_ingredient = None;
                        }
                    } else {
                        toasts.add(new_error_toast("Couldn't find ingredient"));
                    }
                }
            });
    }

    fn update_do_search(
        &mut self,
        conn: &mut database::Connection,
        mut search_for_ingredients: impl FnMut(
            &mut database::Connection,
            IngredientSearchControl,
            Vec<IngredientHandle>,
        ),
        ui: &mut egui::Ui,
    ) {
        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder())
            .size(egui_extras::Size::exact(50.0))
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("for recipes including");
                        egui::ComboBox::from_id_salt("recipe search combo-box")
                            .selected_text(self.control.to_string())
                            .show_ui(ui, |ui| {
                                for c in IngredientSearchControl::iter() {
                                    let s = c.to_string();
                                    ui.selectable_value(&mut self.control, c, s);
                                }
                            });
                        if let IngredientSearchControl::AtLeast(v) = &mut self.control {
                            ui.add(egui::DragValue::new(v).speed(1));
                        }
                        ui.label("of the listed ingredient");
                    });
                });
                strip.cell(|ui| {
                    if ui
                        .add_enabled(!self.to_search.is_empty(), egui::Button::new("Search"))
                        .clicked()
                    {
                        search_for_ingredients(conn, self.control, self.to_search.clone());
                    }
                });
            });
    }

    fn update(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        search_for_ingredients: impl FnMut(
            &mut database::Connection,
            IngredientSearchControl,
            Vec<IngredientHandle>,
        ),
        ui: &mut egui::Ui,
    ) {
        let style = ui.style();
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;

        let separator_height = 6.0;
        let add_ingredient_height = button_height + spacing + separator_height + spacing;
        let search_height = button_height + spacing + separator_height;

        egui_extras::StripBuilder::new(ui)
            .size(egui_extras::Size::remainder())
            .size(egui_extras::Size::exact(add_ingredient_height))
            .size(egui_extras::Size::exact(search_height))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        self.update_table(ui);
                    });
                });
                strip.cell(|ui| {
                    ui.separator();
                    self.update_add_ingredient(conn, toasts, ui);
                });
                strip.cell(|ui| {
                    ui.separator();
                    self.update_do_search(conn, search_for_ingredients, ui);
                });
            });
    }

    fn ingredient_deleted(&mut self, id: IngredientId) {
        self.new_ingredient = None;
        self.cached_ingredient_search = None;
        self.to_search.retain(|i| i.id != id);
    }
}

struct RecipeSearchByName {
    name: String,
    recipes: Option<query::CachedQuery<RecipeId>>,
}

impl RecipeSearchByName {
    fn new() -> Self {
        Self {
            name: "".into(),
            recipes: None,
        }
    }

    fn update(
        &mut self,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
        ui: &mut egui::Ui,
    ) {
        ui.add(
            egui::TextEdit::singleline(&mut self.name)
                .hint_text("search by name")
                .desired_width(f32::INFINITY),
        );
        query::search_recipes(conn, &mut self.recipes, &self.name);

        let available_height = ui.available_height();
        egui_extras::TableBuilder::new(ui)
            .id_salt("recipe search results table")
            .striped(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(egui_extras::Column::remainder())
            .min_scrolled_height(0.0)
            .max_scroll_height(available_height)
            .body(|mut body| {
                let recipe_iter = self
                    .recipes
                    .as_ref()
                    .map(|c| c.results.iter())
                    .into_iter()
                    .flatten();
                for (id, name) in recipe_iter {
                    body.row(20.0, |mut row| {
                        let mut shown = recipe_windows.contains_key(&id);
                        row.col(|ui| {
                            ui.toggle_value(&mut shown, name.clone());
                        });

                        if shown && !recipe_windows.contains_key(&id) {
                            recipe_windows.insert(*id, RecipeWindow::new(conn, *id, false));
                        } else if !shown {
                            recipe_windows.remove(id);
                        }
                    });
                }
            });
    }

    fn recipe_deleted(&mut self, to_delete: RecipeId) {
        if let Some(c) = &mut self.recipes {
            c.results.retain(|(id, _)| *id != to_delete);
        }
    }
}

#[derive(Copy, Clone, EnumIter, Display, Default, PartialEq, Eq)]
enum RecipeSearchTab {
    #[display("By Name")]
    #[default]
    ByName,
    #[display("By Ingredient")]
    ByIngredient,
}

pub struct RecipeSearchWindow {
    selected_tab: RecipeSearchTab,
    by_ingredient: RecipeSearchByIngredient,
    by_name: RecipeSearchByName,
}

impl RecipeSearchWindow {
    pub fn new() -> Self {
        Self {
            selected_tab: Default::default(),
            by_ingredient: RecipeSearchByIngredient::new(),
            by_name: RecipeSearchByName::new(),
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
        toasts: &mut egui_toast::Toasts,
        search_for_ingredients: impl FnMut(
            &mut database::Connection,
            IngredientSearchControl,
            Vec<IngredientHandle>,
        ),
    ) -> bool {
        let mut open = true;
        egui::Window::new("Recipe Search")
            .open(&mut open)
            .default_height(200.0)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for v in RecipeSearchTab::iter() {
                        ui.selectable_value(&mut self.selected_tab, v, v.to_string());
                    }
                });
                ui.separator();
                match self.selected_tab {
                    RecipeSearchTab::ByIngredient => {
                        self.by_ingredient
                            .update(conn, toasts, search_for_ingredients, ui);
                    }
                    RecipeSearchTab::ByName => {
                        self.by_name.update(conn, recipe_windows, ui);
                    }
                }
            });
        !open
    }

    pub fn recipe_deleted(&mut self, id: RecipeId) {
        self.by_name.recipe_deleted(id);
    }

    pub fn ingredient_deleted(&mut self, id: IngredientId) {
        self.by_ingredient.ingredient_deleted(id);
    }
}
