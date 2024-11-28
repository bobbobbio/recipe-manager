// Copyright 2023 Remi Bernotavicius

use crate::import;
use diesel::BelongingToDsl as _;
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use eframe::egui;
use std::collections::{BTreeMap, HashMap};
use std::mem;

use crate::database;
use crate::database::models::{
    Ingredient, IngredientMeasurement, IngredientUsage, IngredientUsageId, Recipe, RecipeCategory,
    RecipeCategoryId, RecipeHandle, RecipeId,
};

struct CategoryListWindow {
    categories: HashMap<RecipeCategoryId, (RecipeCategory, bool)>,
    new_category_name: String,
}

impl CategoryListWindow {
    fn new(conn: &mut database::Connection) -> Self {
        use database::schema::recipe_categories::dsl::*;
        Self {
            categories: recipe_categories
                .select(RecipeCategory::as_select())
                .load(conn)
                .unwrap()
                .into_iter()
                .map(|cat| (cat.id, (cat, false)))
                .collect(),
            new_category_name: String::new(),
        }
    }

    fn add_category(&mut self, conn: &mut database::Connection) {
        use database::schema::recipe_categories::dsl::*;
        use diesel::insert_into;

        insert_into(recipe_categories)
            .values(name.eq(&self.new_category_name))
            .execute(conn)
            .unwrap();

        *self = Self::new(conn);
    }

    fn update(&mut self, ctx: &egui::Context, conn: &mut database::Connection) {
        egui::Window::new("Categories").show(ctx, |ui| {
            let scroll_height = ui.available_height() - 30.0;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(scroll_height)
                .show(ui, |ui| {
                    let mut sorted_categories: Vec<_> = self.categories.values_mut().collect();
                    sorted_categories.sort_by(|a, b| a.0.name.cmp(&b.0.name));

                    for (cat, shown) in sorted_categories {
                        ui.toggle_value(shown, cat.name.clone());
                    }
                });
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.new_category_name)
                        .desired_width(ui.available_width() - 50.0),
                );
                if ui.button("Add").clicked() {
                    self.add_category(conn)
                }
            });
        });
    }
}

struct RecipeListWindow {
    name: String,
    recipes: Vec<RecipeHandle>,
}

impl RecipeListWindow {
    fn new(conn: &mut database::Connection, recipe_category: RecipeCategory) -> Self {
        use database::schema::recipes::dsl::*;
        Self {
            name: recipe_category.name,
            recipes: recipes
                .select(RecipeHandle::as_select())
                .filter(category.eq(recipe_category.id))
                .load(conn)
                .unwrap(),
        }
    }

    fn update(&self, ctx: &egui::Context) -> (bool, Vec<RecipeId>) {
        let mut open = true;
        let mut recipes_to_show = vec![];
        egui::Window::new(self.name.clone())
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for recipe in &self.recipes {
                        if ui.button(recipe.name.clone()).clicked() {
                            recipes_to_show.push(recipe.id);
                        }
                    }
                });
            });
        (!open, recipes_to_show)
    }
}

struct IngredientBeingEdited {
    id: IngredientUsageId,
    name: String,
    category: String,
    quantity: String,
    quantity_units: Option<IngredientMeasurement>,
}

impl IngredientBeingEdited {
    fn new(i: &Ingredient, u: &IngredientUsage) -> Self {
        Self {
            id: u.id,
            name: i.name.clone(),
            category: i.category.as_deref().unwrap_or("").into(),
            quantity: u.quantity.to_string(),
            quantity_units: u.quantity_units,
        }
    }
}

struct RecipeWindow {
    recipe: Recipe,
    ingredients: Vec<(IngredientUsage, Ingredient)>,
    ingredient_being_edited: Option<IngredientBeingEdited>,
}

impl RecipeWindow {
    fn new(conn: &mut database::Connection, recipe_id: RecipeId) -> Self {
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
        }
    }

    fn delete_recipe_ingredient(conn: &mut database::Connection, usage_id: IngredientUsageId) {
        use database::schema::ingredient_usages::dsl::*;
        use diesel::delete;

        delete(ingredient_usages)
            .filter(id.eq(usage_id))
            .execute(conn)
            .unwrap();
    }

    fn edit_recipe_ingredient(
        conn: &mut database::Connection,
        usage_id: IngredientUsageId,
        new_ingredient: &Ingredient,
        new_quantity: f32,
        new_quantity_units: Option<IngredientMeasurement>,
    ) {
        use database::schema::ingredient_usages::dsl::*;
        use diesel::update;

        update(ingredient_usages)
            .filter(id.eq(usage_id))
            .set((
                ingredient_id.eq(new_ingredient.id),
                quantity.eq(new_quantity),
                quantity_units.eq(new_quantity_units),
            ))
            .execute(conn)
            .unwrap();
    }

    fn update_ingredients(
        &mut self,
        conn: &mut database::Connection,
        all_ingredients: &BTreeMap<String, Ingredient>,
        ui: &mut egui::Ui,
    ) {
        let mut refresh_self = false;
        let name = &self.recipe.name;
        egui::Grid::new(format!("{name} ingredients")).show(ui, |ui| {
            ui.label("name");
            ui.label("category");
            ui.label("quantity");
            ui.label("measurement");
            ui.end_row();

            for (usage, ingredient) in &self.ingredients {
                if let Some(e) = &mut self.ingredient_being_edited {
                    if e.id == usage.id {
                        ui.add(egui_dropdown::DropDownBox::from_iter(
                            all_ingredients.keys(),
                            "ingredient",
                            &mut e.name,
                            |ui, text| ui.selectable_label(false, text),
                        ));
                        if let Some(i) = all_ingredients.get(&e.name) {
                            e.category = i.category.clone().unwrap_or_default();
                        }
                        ui.label(&e.category);
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
                        if ui.button("save").clicked() && all_ingredients.contains_key(&e.name) {
                            Self::edit_recipe_ingredient(
                                conn,
                                e.id,
                                all_ingredients.get(&e.name).unwrap(),
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
                if self.ingredient_being_edited.is_none() {
                    if ui.button("edit").clicked() {
                        self.ingredient_being_edited =
                            Some(IngredientBeingEdited::new(ingredient, usage));
                    }
                    if ui.button("delete").clicked() {
                        Self::delete_recipe_ingredient(conn, usage.id);
                        refresh_self = true;
                    }
                }
                ui.end_row();
            }
        });

        if refresh_self {
            *self = Self::new(conn, self.recipe.id);
        }
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        conn: &mut database::Connection,
        all_ingredients: &BTreeMap<String, Ingredient>,
    ) -> bool {
        let mut open = true;
        egui::Window::new(self.recipe.name.clone())
            .open(&mut open)
            .show(ctx, |ui| {
                self.update_ingredients(conn, all_ingredients, ui);
                ui.label("duration:");
                let mut duration = format!("{:?}", &self.recipe.duration);
                ui.add(egui::TextEdit::singleline(&mut duration));
                ui.label("description:");
                let mut description = self.recipe.description.clone();
                ui.add(egui::TextEdit::singleline(&mut description));
            });
        !open
    }
}

#[derive(Default)]
enum ImportWindow {
    #[default]
    Ready,
    Running {
        importer: crate::import::Importer,
    },
    Failed {
        error: crate::Error,
    },
    Success {
        num_imported: usize,
    },
}

impl ImportWindow {
    fn update(&mut self, conn: &mut database::Connection, ctx: &egui::Context) -> bool {
        let mut open = true;
        egui::Window::new("Import data")
            .open(&mut open)
            .show(ctx, |ui| {
                let next = match self {
                    Self::Ready => Self::update_ready(ui),
                    Self::Running { importer } => {
                        ctx.request_repaint_after(std::time::Duration::from_millis(0));
                        Self::update_running(conn, importer, ui)
                    }
                    Self::Failed { error } => Self::update_failed(error, ui),
                    Self::Success { num_imported } => Self::update_success(*num_imported, ui),
                };
                if let Some(next) = next {
                    *self = next;
                }
            });
        !open
    }

    fn update_ready(ui: &mut egui::Ui) -> Option<Self> {
        if ui.button("import data").clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter("recipebook", &["recipebook"])
                .set_directory("/")
                .pick_file()
            {
                return Some(match import::Importer::new(file) {
                    Ok(importer) => Self::Running { importer },
                    Err(error) => Self::Failed { error },
                });
            }
        }
        None
    }

    fn update_running(
        conn: &mut database::Connection,
        importer: &mut import::Importer,
        ui: &mut egui::Ui,
    ) -> Option<Self> {
        ui.label("importing data..");
        ui.add(egui::widgets::ProgressBar::new(importer.percent_done()));

        if !importer.done() {
            if let Err(error) = importer.import_one(conn) {
                return Some(Self::Failed { error });
            }
        } else {
            return Some(Self::Success {
                num_imported: importer.num_imported(),
            });
        }

        None
    }

    fn update_failed(error: &crate::Error, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!("import failed with error: {error}"));
        ui.button("okay").clicked().then_some(Self::Ready)
    }

    fn update_success(num_imported: usize, ui: &mut egui::Ui) -> Option<Self> {
        ui.label(format!(
            "import succeeded. {num_imported} recipes imported."
        ));
        ui.button("okay").clicked().then_some(Self::Ready)
    }
}

pub struct RecipeManager {
    category_list: CategoryListWindow,
    conn: database::Connection,
    import_window: Option<ImportWindow>,
    recipe_lists: HashMap<RecipeCategoryId, RecipeListWindow>,
    recipes: Vec<RecipeWindow>,
    all_ingredients: BTreeMap<String, Ingredient>,
}

impl RecipeManager {
    pub fn new(mut conn: database::Connection) -> Self {
        use database::schema::ingredients::dsl::*;
        let all_ingredients = ingredients
            .select(Ingredient::as_select())
            .load(&mut conn)
            .unwrap();

        Self {
            category_list: CategoryListWindow::new(&mut conn),
            conn,
            import_window: None,
            recipe_lists: Default::default(),
            recipes: Default::default(),
            all_ingredients: all_ingredients
                .into_iter()
                .map(|i| (i.name.clone(), i))
                .collect(),
        }
    }

    fn update_category_list_window(&mut self, ctx: &egui::Context) {
        self.category_list.update(ctx, &mut self.conn);
        let categories: HashMap<_, _> = self
            .category_list
            .categories
            .values()
            .filter(|(_, shown)| *shown)
            .map(|(cat, _)| (cat.id.clone(), cat.clone()))
            .collect();
        self.recipe_lists
            .retain(|key, _| categories.contains_key(key));
        for (_, cat) in categories {
            if !self.recipe_lists.contains_key(&cat.id) {
                self.recipe_lists
                    .insert(cat.id, RecipeListWindow::new(&mut self.conn, cat));
            }
        }
    }

    fn show_recipe(&mut self, recipe_id: RecipeId) {
        self.recipes
            .push(RecipeWindow::new(&mut self.conn, recipe_id));
    }

    fn update_recipe_list_windows(&mut self, ctx: &egui::Context) {
        for (id, list) in mem::take(&mut self.recipe_lists) {
            let (closed, recipes_shown) = list.update(ctx);
            for recipe_id in recipes_shown {
                self.show_recipe(recipe_id);
            }
            if !closed {
                self.recipe_lists.insert(id, list);
            } else {
                self.category_list.categories.get_mut(&id).unwrap().1 = false;
            }
        }
    }

    fn update_recipes(&mut self, ctx: &egui::Context) {
        for mut recipe in mem::take(&mut self.recipes) {
            let closed = recipe.update(ctx, &mut self.conn, &self.all_ingredients);
            if !closed {
                self.recipes.push(recipe);
            }
        }
    }

    fn update_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import").clicked() && self.import_window.is_none() {
                        self.import_window = Some(ImportWindow::default());
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
}

impl eframe::App for RecipeManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_menu(ctx);
        self.update_import_window(ctx);
        self.update_category_list_window(ctx);
        self.update_recipe_list_windows(ctx);
        self.update_recipes(ctx);
    }
}
