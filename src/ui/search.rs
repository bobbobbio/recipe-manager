use super::{new_error_toast, query, recipe::RecipeWindow};
use crate::database::{
    self,
    models::{Ingredient, IngredientHandle, RecipeHandle, RecipeId},
};
use derive_more::Display;
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

    fn update_table(
        &mut self,
        conn: &mut database::Connection,
        recipe_windows: &mut HashMap<RecipeId, RecipeWindow>,
        ui: &mut egui::Ui,
    ) {
        if self.results.is_empty() {
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

pub struct RecipeSearchWindow {
    to_search: Vec<IngredientHandle>,

    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,
    control: IngredientSearchControl,
}

impl RecipeSearchWindow {
    pub fn new() -> Self {
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
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Ingredient");
                });
                header.col(|ui| {
                    ui.heading("");
                });
            })
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
                strip.cell(|ui| {
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
                        .hint_text("search for ingredient")
                        .desired_width(f32::INFINITY),
                    );
                });
                strip.cell(|ui| {
                    if ui.button("Add").clicked() {
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
            if ui
                .add_enabled(!self.to_search.is_empty(), egui::Button::new("Search"))
                .clicked()
            {
                search_for_ingredients(conn, self.control, self.to_search.clone());
            }
        });
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        search_for_ingredients: impl FnMut(
            &mut database::Connection,
            IngredientSearchControl,
            Vec<IngredientHandle>,
        ),
    ) -> bool {
        let style = ctx.style();
        let button_height = (egui::TextStyle::Button.resolve(&style).size
            + style.spacing.button_padding.y as f32 * 2.0)
            .max(style.spacing.interact_size.y);
        let spacing = style.spacing.item_spacing.y;

        let separator_height = 6.0;
        let add_ingredient_height = button_height + spacing + separator_height + spacing;
        let search_height = button_height + spacing + separator_height;

        let mut open = true;
        egui::Window::new("Recipe Search")
            .open(&mut open)
            .default_height(200.0)
            .default_width(300.0)
            .show(ctx, |ui| {
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
                    })
            });
        !open
    }
}
