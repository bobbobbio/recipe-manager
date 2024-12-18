use super::{
    calendar::{this_week, RecipeWeek},
    new_error_toast, query,
    search::SearchWidget,
    unit_conversion,
};
use crate::database;
use crate::database::models::{
    Ingredient, IngredientCaloriesEntry, IngredientMeasurement, IngredientUsageId, Recipe,
    RecipeCategoryId, RecipeDuration, RecipeId,
};
use eframe::egui;

struct IngredientBeingEdited {
    usage_id: IngredientUsageId,
    new_ingredient_name: String,
    ingredient: Option<Ingredient>,
    quantity: String,
    quantity_units: Option<IngredientMeasurement>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,
}

impl IngredientBeingEdited {
    fn new(usage: &RecipeIngredient) -> Self {
        Self {
            usage_id: usage.id,
            new_ingredient_name: usage.ingredient.name.clone(),
            ingredient: Some(usage.ingredient.clone()),
            quantity: usage.quantity.to_string(),
            quantity_units: usage.quantity_units,
            cached_ingredient_search: None,
        }
    }
}

pub struct RecipeIngredient {
    pub id: IngredientUsageId,
    pub ingredient: Ingredient,
    pub quantity: f32,
    pub quantity_units: Option<IngredientMeasurement>,
    pub calories: Vec<IngredientCaloriesEntry>,
}

impl RecipeIngredient {
    fn calories(&self) -> Option<f32> {
        use unit_conversion::{conversion_factor, MeasurementKind};

        for c in &self.calories {
            if c.quantity_units == self.quantity_units {
                return Some(c.calories * self.quantity);
            }
        }
        for c in &self.calories {
            if let (Some(a), Some(b)) = (self.quantity_units, c.quantity_units) {
                if MeasurementKind::from(a) == MeasurementKind::from(b) {
                    return Some(c.calories * conversion_factor(a, b) * self.quantity / c.quantity);
                }
            }
        }
        None
    }
}

pub enum UpdateEvent {
    Closed,
    Renamed(Recipe),
    Scheduled(chrono::NaiveWeek),
    CategoryChanged,
}

pub struct RecipeWindow {
    recipe: Recipe,

    ingredients: Vec<RecipeIngredient>,
    ingredient_being_edited: Option<IngredientBeingEdited>,

    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,

    week: RecipeWeek,

    new_category_name: String,
    new_category: Option<RecipeCategoryId>,
    cached_category_search: Option<query::CachedQuery<RecipeCategoryId>>,

    edit_mode: bool,
}

impl RecipeWindow {
    pub fn new(conn: &mut database::Connection, recipe_id: RecipeId, edit_mode: bool) -> Self {
        let (recipe, category_name, ingredients) = query::get_recipe(conn, recipe_id);
        Self {
            recipe,

            ingredients,
            ingredient_being_edited: None,

            new_ingredient_name: String::new(),
            new_ingredient: None,
            cached_ingredient_search: None,

            week: RecipeWeek::new(conn, this_week()),

            new_category_name: category_name,
            new_category: None,
            cached_category_search: None,

            edit_mode,
        }
    }

    fn update_ingredients(
        &mut self,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
        ui: &mut egui::Ui,
    ) {
        let mut refresh_self = false;
        egui::Grid::new(("ingredient grid", self.recipe.id)).show(ui, |ui| {
            ui.label("Name");
            ui.label("Category");
            ui.label("Quantity");
            ui.label("Measurement");
            ui.label("Calories");
            ui.end_row();

            for usage in &self.ingredients {
                if let Some(e) = &mut self.ingredient_being_edited {
                    if e.usage_id == usage.id {
                        ui.add(SearchWidget::new(
                            e.usage_id,
                            &mut e.new_ingredient_name,
                            &mut e.ingredient,
                            |query| {
                                query::search_ingredients(
                                    conn,
                                    &mut e.cached_ingredient_search,
                                    query,
                                )
                            },
                        ));

                        if let Some(Ingredient {
                            category: Some(category),
                            ..
                        }) = &e.ingredient
                        {
                            ui.label(category.as_str());
                        } else {
                            ui.label("");
                        }
                        ui.add(egui::TextEdit::singleline(&mut e.quantity));
                        egui::ComboBox::from_id_salt((
                            "recipe ingredient quantity units",
                            self.recipe.id,
                        ))
                        .selected_text(e.quantity_units.as_ref().map(|q| q.as_str()).unwrap_or(""))
                        .show_ui(ui, |ui| {
                            for m in IngredientMeasurement::iter() {
                                ui.selectable_value(&mut e.quantity_units, Some(m), m.as_str());
                            }
                            ui.selectable_value(&mut e.quantity_units, None, "");
                        });
                        ui.label("");
                        if ui.button("Save").clicked() {
                            if e.ingredient.is_some() {
                                query::edit_recipe_ingredient(
                                    conn,
                                    e.usage_id,
                                    e.ingredient.as_ref().unwrap(),
                                    e.quantity.parse().unwrap_or(0.0),
                                    e.quantity_units,
                                );
                                refresh_self = true;
                            } else {
                                toasts.add(new_error_toast("Couldn't find ingredient"));
                            }
                        }
                        ui.end_row();
                        continue;
                    }
                }

                ui.label(&usage.ingredient.name);
                ui.label(usage.ingredient.category.as_deref().unwrap_or(""));
                ui.label(usage.quantity.to_string());
                ui.label(
                    usage
                        .quantity_units
                        .as_ref()
                        .map(|c| c.as_str())
                        .unwrap_or(""),
                );
                ui.label(
                    usage
                        .calories()
                        .map(|c| format!("{c:.2}"))
                        .unwrap_or_default(),
                );
                if self.edit_mode && self.ingredient_being_edited.is_none() {
                    if ui.button("Edit").clicked() {
                        self.ingredient_being_edited = Some(IngredientBeingEdited::new(usage));
                    }
                    if ui.button("Delete").clicked() {
                        query::delete_recipe_ingredient(conn, usage.id);
                        refresh_self = true;
                    }
                }
                ui.end_row();
            }
        });

        if self.edit_mode {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ui.label("Add Ingredient:");
                ui.add(
                    SearchWidget::new(
                        "ingredient",
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
                        query::add_recipe_ingredient(conn, self.recipe.id, ingredient.id, 1.0);
                        self.new_ingredient_name = "".into();
                        self.new_ingredient = None;
                        refresh_self = true;
                    } else {
                        toasts.add(new_error_toast("Couldn't find ingredient"));
                    }
                }
            });
        }

        if refresh_self {
            *self = Self::new(conn, self.recipe.id, self.edit_mode);
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        toasts: &mut egui_toast::Toasts,
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        let mut open = true;
        egui::Window::new(self.recipe.name.clone())
            .id(egui::Id::new(("recipe", self.recipe.id)))
            .open(&mut open)
            .show(ctx, |ui| {
                self.update_ingredients(conn, toasts, ui);
                egui::Grid::new(("recipe information", self.recipe.id)).show(ui, |ui| {
                    if self.edit_mode {
                        ui.label("Name:");
                        let mut name = self.recipe.name.clone();
                        ui.add(egui::TextEdit::singleline(&mut name));
                        if name != self.recipe.name {
                            query::edit_recipe_name(conn, self.recipe.id, &name);
                            self.recipe.name = name.clone();
                            events.push(UpdateEvent::Renamed(self.recipe.clone()));
                        }
                        ui.end_row();

                        ui.label("Category:");
                        ui.add(
                            SearchWidget::new(
                                ("recipe category", self.recipe.id),
                                &mut self.new_category_name,
                                &mut self.new_category,
                                |query| {
                                    query::search_recipe_categories(
                                        conn,
                                        &mut self.cached_category_search,
                                        query,
                                    )
                                },
                            )
                            .hint_text("search for category"),
                        );
                        if ui.button("Save").clicked() {
                            if let Some(cat) = self.new_category {
                                query::edit_recipe_category(conn, self.recipe.id, cat);
                                events.push(UpdateEvent::CategoryChanged);
                            } else {
                                toasts.add(new_error_toast("Couldn't find recipe category"));
                            }
                        }
                        ui.end_row();
                    }
                    ui.label("Duration:");
                    if self.edit_mode {
                        let mut selected = self.recipe.duration.clone();
                        egui::ComboBox::from_id_salt(("recipe duration", self.recipe.id))
                            .selected_text(&selected.to_string())
                            .show_ui(ui, |ui| {
                                for d in RecipeDuration::iter() {
                                    ui.selectable_value(&mut selected, d, d.to_string());
                                }
                            });
                        if selected != self.recipe.duration {
                            query::edit_recipe_duration(conn, self.recipe.id, selected);
                            self.recipe.duration = selected;
                        }
                    } else {
                        ui.label(self.recipe.duration.to_string());
                    }
                    ui.end_row();

                    ui.label("Description:");
                    if self.edit_mode {
                        let mut description = self.recipe.description.clone();
                        ui.add(egui::TextEdit::multiline(&mut description));
                        if description != self.recipe.description {
                            query::edit_recipe_description(conn, self.recipe.id, &description);
                            self.recipe.description = description;
                        }
                    } else {
                        ui.label(&self.recipe.description);
                    }
                    ui.end_row();

                    ui.label(format!(
                        "Total Calories: {:.2}",
                        self.ingredients
                            .iter()
                            .filter_map(|i| i.calories())
                            .sum::<f32>()
                    ));
                    ui.end_row();
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    if !self.edit_mode {
                        self.ingredient_being_edited = None;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("Schedule", |ui| {
                            for (day, recipe) in self.week.recipes() {
                                let recipe =
                                    recipe.map(|r| r.name.clone()).unwrap_or("No Recipe".into());
                                if ui.button(format!("{day}: {recipe}")).clicked() {
                                    self.week.schedule(conn, day, self.recipe.id);
                                    ui.close_menu();
                                    events.push(UpdateEvent::Scheduled(*self.week.week()));
                                }
                            }
                        });
                        self.week.pick_date(conn, |date| {
                            ui.add(egui_extras::DatePickerButton::new(date));
                        });
                    });
                });
            });

        if !open {
            events.push(UpdateEvent::Closed);
        }
        events
    }

    pub fn recipe_scheduled(&mut self, conn: &mut database::Connection, week: chrono::NaiveWeek) {
        if self.week.week() == &week {
            self.week.refresh(conn);
        }
    }

    pub fn ingredient_edited(&mut self, conn: &mut database::Connection) {
        *self = Self::new(conn, self.recipe.id, self.edit_mode);
    }
}
