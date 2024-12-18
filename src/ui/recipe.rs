use super::{
    calendar::{this_week, RecipeWeek},
    new_error_toast, query,
    search::SearchWidget,
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

#[derive(PartialEq, Eq, Debug)]
enum MeasurementKind {
    Volume,
    Weight,
}

impl From<IngredientMeasurement> for MeasurementKind {
    fn from(m: IngredientMeasurement) -> Self {
        match m {
            IngredientMeasurement::Cups => Self::Volume,
            IngredientMeasurement::FluidOunces => Self::Volume,
            IngredientMeasurement::Grams => Self::Weight,
            IngredientMeasurement::Kilograms => Self::Weight,
            IngredientMeasurement::Kiloliters => Self::Volume,
            IngredientMeasurement::Liters => Self::Volume,
            IngredientMeasurement::Milligrams => Self::Weight,
            IngredientMeasurement::Milliliters => Self::Volume,
            IngredientMeasurement::Ounces => Self::Weight,
            IngredientMeasurement::Pounds => Self::Weight,
            IngredientMeasurement::Tablespoons => Self::Volume,
            IngredientMeasurement::Teaspoons => Self::Volume,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum MeasurementClass {
    Us,
    Metric,
}

impl From<IngredientMeasurement> for MeasurementClass {
    fn from(m: IngredientMeasurement) -> Self {
        match m {
            IngredientMeasurement::Cups => Self::Us,
            IngredientMeasurement::FluidOunces => Self::Us,
            IngredientMeasurement::Grams => Self::Metric,
            IngredientMeasurement::Kilograms => Self::Metric,
            IngredientMeasurement::Kiloliters => Self::Metric,
            IngredientMeasurement::Liters => Self::Metric,
            IngredientMeasurement::Milligrams => Self::Metric,
            IngredientMeasurement::Milliliters => Self::Metric,
            IngredientMeasurement::Ounces => Self::Us,
            IngredientMeasurement::Pounds => Self::Us,
            IngredientMeasurement::Tablespoons => Self::Us,
            IngredientMeasurement::Teaspoons => Self::Us,
        }
    }
}

fn as_teaspoons(a: IngredientMeasurement) -> f32 {
    match a {
        IngredientMeasurement::Cups => 48.0,
        IngredientMeasurement::FluidOunces => 6.0,
        IngredientMeasurement::Teaspoons => 1.0,
        IngredientMeasurement::Tablespoons => 3.0,
        _ => unreachable!(),
    }
}

fn as_milliliters(a: IngredientMeasurement) -> f32 {
    match a {
        IngredientMeasurement::Cups => 236.588236,
        IngredientMeasurement::FluidOunces => 29.573535296,
        IngredientMeasurement::Kiloliters => 1_000_000.0,
        IngredientMeasurement::Liters => 1_000.0,
        IngredientMeasurement::Milliliters => 1.0,
        IngredientMeasurement::Tablespoons => 14.7867648,
        IngredientMeasurement::Teaspoons => 4.92892159,
        _ => unreachable!(),
    }
}

fn as_ounces(a: IngredientMeasurement) -> f32 {
    match a {
        IngredientMeasurement::Ounces => 1.0,
        IngredientMeasurement::Pounds => 16.0,
        _ => unreachable!(),
    }
}

fn as_milligrams(a: IngredientMeasurement) -> f32 {
    match a {
        IngredientMeasurement::Grams => 1_000.0,
        IngredientMeasurement::Kilograms => 1_000_000.0,
        IngredientMeasurement::Milligrams => 1.0,
        IngredientMeasurement::Ounces => 28349.52,
        IngredientMeasurement::Pounds => 453592.4,
        _ => unreachable!(),
    }
}

fn conversion_factor(a: IngredientMeasurement, b: IngredientMeasurement) -> f32 {
    let a_kind = MeasurementKind::from(a);
    let b_kind = MeasurementKind::from(a);
    assert_eq!(a_kind, b_kind);

    let a_class = MeasurementClass::from(a);
    let b_class = MeasurementClass::from(b);

    match a_kind {
        MeasurementKind::Volume => match (a_class, b_class) {
            (MeasurementClass::Us, MeasurementClass::Us) => as_teaspoons(a) / as_teaspoons(b),
            _ => as_milliliters(a) / as_milliliters(b),
        },
        MeasurementKind::Weight => match (a_class, b_class) {
            (MeasurementClass::Us, MeasurementClass::Us) => as_ounces(a) / as_ounces(b),
            _ => as_milligrams(a) / as_milligrams(b),
        },
    }
}

#[test]
fn unit_conversion_us() {
    use IngredientMeasurement::*;
    assert_eq!(conversion_factor(Cups, FluidOunces), 8.0);
    assert_eq!(conversion_factor(Cups, Tablespoons), 16.0);
    assert_eq!(conversion_factor(Cups, Teaspoons), 48.0);

    assert_eq!(conversion_factor(FluidOunces, Cups), 1.0 / 8.0);
    assert_eq!(conversion_factor(Tablespoons, Cups), 1.0 / 16.0);
    assert_eq!(conversion_factor(Teaspoons, Cups), 1.0 / 48.0);

    assert_eq!(conversion_factor(Tablespoons, FluidOunces), 1.0 / 2.0);
    assert_eq!(conversion_factor(Tablespoons, Teaspoons), 3.0);

    assert_eq!(conversion_factor(FluidOunces, Tablespoons), 2.0);
    assert_eq!(conversion_factor(Teaspoons, Tablespoons), 1.0 / 3.0);

    assert_eq!(conversion_factor(Teaspoons, FluidOunces), 1.0 / 6.0);
    assert_eq!(conversion_factor(FluidOunces, Teaspoons), 6.0);

    assert_eq!(conversion_factor(Pounds, Ounces), 16.0);
    assert_eq!(conversion_factor(Ounces, Pounds), 1.0 / 16.0);
}

#[test]
fn unit_conversion_metric() {
    use IngredientMeasurement::*;

    assert_eq!(conversion_factor(Liters, Milliliters), 1_000.0);
    assert_eq!(conversion_factor(Kiloliters, Milliliters), 1_000_000.0);

    assert_eq!(conversion_factor(Milliliters, Liters), 1.0 / 1_000.0);
    assert_eq!(
        conversion_factor(Milliliters, Kiloliters),
        1.0 / 1_000_000.0
    );

    assert_eq!(conversion_factor(Kiloliters, Liters), 1_000.0);
    assert_eq!(conversion_factor(Liters, Kiloliters), 1.0 / 1_000.0);

    assert_eq!(conversion_factor(Grams, Milligrams), 1_000.0);
    assert_eq!(conversion_factor(Kilograms, Milligrams), 1_000_000.0);

    assert_eq!(conversion_factor(Milligrams, Grams), 1.0 / 1_000.0);
    assert_eq!(conversion_factor(Milligrams, Kilograms), 1.0 / 1_000_000.0);

    assert_eq!(conversion_factor(Kilograms, Grams), 1_000.0);
    assert_eq!(conversion_factor(Grams, Kilograms), 1.0 / 1_000.0);
}

#[test]
fn unit_conversion_us_metric() {
    use IngredientMeasurement::*;

    assert_eq!(conversion_factor(Liters, Teaspoons), 202.88412);
    assert_eq!(conversion_factor(Liters, Cups), 4.2267528);
    assert_eq!(conversion_factor(Kiloliters, Teaspoons), 202884.13);

    assert_eq!(conversion_factor(Ounces, Grams), 28.34952);
    assert_eq!(conversion_factor(Pounds, Grams), 453.5924);
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
        let name = &self.recipe.name;
        egui::Grid::new(format!("{name} ingredients")).show(ui, |ui| {
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
                egui::Grid::new("Recipe Information").show(ui, |ui| {
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
