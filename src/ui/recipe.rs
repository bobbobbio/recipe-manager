use super::{
    calendar::{this_week, RecipeWeek},
    query, SearchWidget,
};
use crate::database;
use crate::database::models::{
    Ingredient, IngredientMeasurement, IngredientUsage, IngredientUsageId, Recipe, RecipeDuration,
    RecipeId,
};
use diesel::BelongingToDsl as _;
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
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
    fn new(i: &Ingredient, u: &IngredientUsage) -> Self {
        Self {
            usage_id: u.id,
            new_ingredient_name: i.name.clone(),
            ingredient: Some(i.clone()),
            quantity: u.quantity.to_string(),
            quantity_units: u.quantity_units,
            cached_ingredient_search: None,
        }
    }
}

pub enum UpdateEvent {
    Closed,
    Renamed(Recipe),
    Scheduled,
}

pub struct RecipeWindow {
    recipe: Recipe,
    ingredients: Vec<(IngredientUsage, Ingredient)>,
    ingredient_being_edited: Option<IngredientBeingEdited>,
    new_ingredient_name: String,
    new_ingredient: Option<Ingredient>,
    edit_mode: bool,
    cached_ingredient_search: Option<query::CachedQuery<Ingredient>>,
    week: RecipeWeek,
}

impl RecipeWindow {
    pub fn new(conn: &mut database::Connection, recipe_id: RecipeId, edit_mode: bool) -> Self {
        use database::schema::recipes::dsl::*;
        let recipe = recipes
            .select(Recipe::as_select())
            .filter(id.eq(recipe_id))
            .get_result(conn)
            .unwrap();
        let ingredients = IngredientUsage::belonging_to(&recipe)
            .inner_join(database::schema::ingredients::table)
            .select((IngredientUsage::as_select(), Ingredient::as_select()))
            .load(conn)
            .unwrap();
        Self {
            recipe,
            ingredients,
            ingredient_being_edited: None,
            new_ingredient_name: String::new(),
            new_ingredient: None,
            edit_mode,
            cached_ingredient_search: None,
            week: RecipeWeek::new(conn, this_week()),
        }
    }

    fn update_ingredients(&mut self, conn: &mut database::Connection, ui: &mut egui::Ui) {
        let mut refresh_self = false;
        let name = &self.recipe.name;
        egui::Grid::new(format!("{name} ingredients")).show(ui, |ui| {
            ui.label("Name");
            ui.label("Category");
            ui.label("Quantity");
            ui.label("Measurement");
            ui.end_row();

            for (usage, ingredient) in &self.ingredients {
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
                        egui::ComboBox::from_id_salt("recipe ingredient quantity units")
                            .selected_text(
                                e.quantity_units.as_ref().map(|q| q.as_str()).unwrap_or(""),
                            )
                            .show_ui(ui, |ui| {
                                for m in IngredientMeasurement::iter() {
                                    ui.selectable_value(&mut e.quantity_units, Some(m), m.as_str());
                                }
                                ui.selectable_value(&mut e.quantity_units, None, "");
                            });
                        if ui.button("Save").clicked() && e.ingredient.is_some() {
                            query::edit_recipe_ingredient(
                                conn,
                                e.usage_id,
                                e.ingredient.as_ref().unwrap(),
                                e.quantity.parse().unwrap_or(0.0),
                                e.quantity_units,
                            );
                            refresh_self = true;
                        }
                        ui.end_row();
                        continue;
                    }
                }

                ui.label(&ingredient.name);
                ui.label(ingredient.category.as_deref().unwrap_or(""));
                ui.label(usage.quantity.to_string());
                ui.label(
                    usage
                        .quantity_units
                        .as_ref()
                        .map(|c| c.as_str())
                        .unwrap_or(""),
                );
                if self.edit_mode && self.ingredient_being_edited.is_none() {
                    if ui.button("Edit").clicked() {
                        self.ingredient_being_edited =
                            Some(IngredientBeingEdited::new(ingredient, usage));
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
                ui.add(SearchWidget::new(
                    "ingredient",
                    &mut self.new_ingredient_name,
                    &mut self.new_ingredient,
                    |query| {
                        query::search_ingredients(conn, &mut self.cached_ingredient_search, query)
                    },
                ));

                if ui.button("Add").clicked() {
                    if let Some(ingredient) = &self.new_ingredient {
                        query::add_recipe_ingredient(conn, self.recipe.id, ingredient.id, 1.0);
                        self.new_ingredient_name = "".into();
                        self.new_ingredient = None;
                        refresh_self = true;
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
    ) -> Vec<UpdateEvent> {
        let mut events = vec![];
        let mut open = true;
        egui::Window::new(self.recipe.name.clone())
            .id(egui::Id::new(("recipe", self.recipe.id)))
            .open(&mut open)
            .show(ctx, |ui| {
                self.update_ingredients(conn, ui);
                egui::Grid::new("Recipe Information")
                    .num_columns(2)
                    .show(ui, |ui| {
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
                        }
                        ui.label("Duration:");
                        if self.edit_mode {
                            let mut selected = self.recipe.duration.clone();
                            egui::ComboBox::from_id_salt("recipe duration")
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
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut self.edit_mode, "Edit");
                    if !self.edit_mode {
                        self.ingredient_being_edited = None;
                    }
                    self.week.pick_date(conn, |date| {
                        ui.add(egui_extras::DatePickerButton::new(date));
                    });

                    ui.menu_button("Schedule", |ui| {
                        for (day, recipe) in self.week.recipes() {
                            let recipe =
                                recipe.map(|r| r.name.clone()).unwrap_or("No Recipe".into());
                            if ui.button(format!("{day}: {recipe}")).clicked() {
                                self.week.schedule(conn, day, self.recipe.id);
                                ui.close_menu();
                                events.push(UpdateEvent::Scheduled);
                            }
                        }
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
}
