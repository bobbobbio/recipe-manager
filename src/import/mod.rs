// Copyright 2023 Remi Bernotavicius

use crate::database;
use crate::Result;
use database::models::{
    Ingredient, IngredientId, IngredientMeasurement, IngredientUsage, IngredientUsageId, Recipe,
    RecipeCategory, RecipeCategoryId, RecipeDuration, RecipeId,
};
use diesel::prelude::OptionalExtension as _;
use diesel::ExpressionMethods as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;
use std::mem;
use std::path::Path;

mod plist;

impl IngredientMeasurement {
    fn import(s: &str) -> Self {
        match s {
            "c." => Self::Cups,
            "fl. oz." => Self::FluidOunces,
            "lb." => Self::Pounds,
            "oz." => Self::Ounces,
            "tbsp." => Self::Tablespoons,
            "tsp." => Self::Teaspoons,
            _ => panic!("couldn't import measurement {s:?}"),
        }
    }
}

impl RecipeDuration {
    fn import(time: &str) -> Self {
        match time {
            "Long" => RecipeDuration::Long,
            "Medium" => RecipeDuration::Medium,
            "Really Long" => RecipeDuration::ReallyLong,
            "Short" => RecipeDuration::Short,
            v => panic!("unexpected value {v:?} for time"),
        }
    }
}

impl Recipe {
    fn import(
        recipe_id: RecipeId,
        recipe_category_id: RecipeCategoryId,
        recipe: plist::Recipe,
    ) -> Self {
        Self {
            id: recipe_id,
            name: recipe.name,
            description: recipe.other,
            duration: RecipeDuration::import(&recipe.time[..]),
            category: recipe_category_id,
        }
    }
}

fn import_ingredient(
    conn: &mut database::Connection,
    plist_ingredient: plist::Ingredient,
    recipe_id: RecipeId,
    ingredient_usage_id: &mut IngredientUsageId,
    ingredient_id: &mut IngredientId,
) -> Result<()> {
    use database::schema::ingredients::dsl::*;

    let new_ingredient_name = plist_ingredient.name.to_lowercase();
    let existing_ingredient = ingredients
        .select(Ingredient::as_select())
        .filter(name.eq(&new_ingredient_name))
        .get_result(conn)
        .optional()
        .unwrap();
    let ingredient_id = if let Some(existing) = existing_ingredient {
        existing.id
    } else {
        let new_id = *ingredient_id;
        let new_ingredient = Ingredient {
            id: new_id,
            name: new_ingredient_name,
            category: (!plist_ingredient.category.is_empty()).then_some(plist_ingredient.category),
        };
        diesel::insert_into(ingredients)
            .values(new_ingredient)
            .execute(conn)
            .unwrap();

        *ingredient_id = ingredient_id.next();
        new_id
    };

    let new_usage = IngredientUsage {
        id: *ingredient_usage_id,
        recipe_id,
        ingredient_id,
        quantity: plist_ingredient.quantity as f32,
        quantity_units: (!plist_ingredient.measurement.trim().is_empty())
            .then(|| IngredientMeasurement::import(&plist_ingredient.measurement)),
    };

    diesel::insert_into(database::schema::ingredient_usages::dsl::ingredient_usages)
        .values(new_usage)
        .execute(conn)
        .unwrap();
    *ingredient_usage_id = ingredient_usage_id.next();

    Ok(())
}

fn import_recipes_from_box(
    conn: &mut database::Connection,
    num_imported: &mut usize,
    recipes: Vec<plist::Recipe>,
    recipe_category_id: RecipeCategoryId,
    recipe_id: &mut RecipeId,
    ingredient_usage_id: &mut IngredientUsageId,
    ingredient_id: &mut IngredientId,
) -> Result<()> {
    for mut plist_recipe in recipes {
        let id = *recipe_id;
        let plist_ingredients = mem::take(&mut plist_recipe.ingredients);
        let new_recipe = Recipe::import(id, recipe_category_id, plist_recipe);
        diesel::insert_into(database::schema::recipes::dsl::recipes)
            .values(new_recipe)
            .execute(conn)
            .unwrap();
        *recipe_id = recipe_id.next();

        for plist_ingredient in plist_ingredients {
            import_ingredient(
                conn,
                plist_ingredient,
                id,
                ingredient_usage_id,
                ingredient_id,
            )?;
        }
        *num_imported += 1;
    }
    Ok(())
}

fn import_recipe_category(
    conn: &mut database::Connection,
    name: String,
    recipe_category_id: &mut RecipeCategoryId,
) -> Result<RecipeCategoryId> {
    let id = *recipe_category_id;
    let new_category = RecipeCategory { id, name };
    diesel::insert_into(database::schema::recipe_categories::dsl::recipe_categories)
        .values(new_category)
        .execute(conn)
        .unwrap();
    *recipe_category_id = recipe_category_id.next();
    Ok(id)
}

pub struct RecipeImporter {
    recipe_boxes: Vec<plist::RecipeBox>,
    working_recipe_box: Option<(RecipeCategoryId, plist::RecipeBox)>,

    num_imported: usize,
    total_num_recipes: usize,

    recipe_category_id_vendor: RecipeCategoryId,
    recipe_id_vendor: RecipeId,
    ingredient_usage_id_vendor: IngredientUsageId,
    ingredient_id_vendor: IngredientId,
}

impl RecipeImporter {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let recipe_boxes = plist::decode_recipes_from_path(path)?;

        let total_num_recipes = recipe_boxes.iter().map(|b| b.recipes.len()).sum();

        Ok(Self {
            recipe_boxes,
            working_recipe_box: None,

            num_imported: 0,
            total_num_recipes,

            recipe_category_id_vendor: RecipeCategoryId::INITIAL,
            recipe_id_vendor: RecipeId::INITIAL,
            ingredient_usage_id_vendor: IngredientUsageId::INITIAL,
            ingredient_id_vendor: IngredientId::INITIAL,
        })
    }

    pub fn done(&self) -> bool {
        self.recipe_boxes.is_empty() && self.working_recipe_box.is_none()
    }

    pub fn num_imported(&self) -> usize {
        self.num_imported
    }

    pub fn percent_done(&self) -> f32 {
        self.num_imported as f32 / self.total_num_recipes as f32
    }

    pub fn import_one(&mut self, conn: &mut database::Connection) -> Result<()> {
        assert!(!self.done());

        if self.working_recipe_box.is_none() {
            let plist_recipe_box = self.recipe_boxes.remove(0);
            let recipe_category_id = import_recipe_category(
                conn,
                plist_recipe_box.name.clone(),
                &mut self.recipe_category_id_vendor,
            )?;
            self.working_recipe_box = Some((recipe_category_id, plist_recipe_box));
        }

        let (recipe_category_id, working) = &mut self.working_recipe_box.as_mut().unwrap();

        const BATCH_SIZE: usize = 5;
        let split_point = working.recipes.len().saturating_sub(BATCH_SIZE);
        let recipe_batch = working.recipes.split_off(split_point);

        import_recipes_from_box(
            conn,
            &mut self.num_imported,
            recipe_batch,
            *recipe_category_id,
            &mut self.recipe_id_vendor,
            &mut self.ingredient_usage_id_vendor,
            &mut self.ingredient_id_vendor,
        )?;

        if working.recipes.is_empty() {
            self.working_recipe_box = None;
        }

        Ok(())
    }
}

pub fn import_recipes(mut conn: database::Connection, path: impl AsRef<Path>) -> Result<()> {
    let mut importer = RecipeImporter::new(path)?;

    while !importer.done() {
        importer.import_one(&mut conn)?;
        println!("imported {}%", importer.percent_done() * 100.0);
    }

    Ok(())
}

pub struct CalendarImporter {
    recipe_weeks: Vec<plist::RecipeWeek>,
    num_imported: usize,
}

impl CalendarImporter {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let recipe_weeks = plist::decode_calendar_from_path(path)?;

        Ok(Self {
            recipe_weeks,
            num_imported: 0,
        })
    }

    pub fn done(&self) -> bool {
        self.recipe_weeks.is_empty()
    }

    #[expect(dead_code)]
    pub fn num_imported(&self) -> usize {
        self.num_imported
    }

    pub fn percent_done(&self) -> f32 {
        self.num_imported as f32 / (self.recipe_weeks.len() + self.num_imported) as f32
    }

    pub fn import_one(&mut self, _conn: &mut database::Connection) -> Result<()> {
        assert!(!self.done());

        Ok(())
    }
}
pub fn import_calendar(mut conn: database::Connection, path: impl AsRef<Path>) -> Result<()> {
    let mut importer = CalendarImporter::new(path)?;

    while !importer.done() {
        importer.import_one(&mut conn)?;
        println!("imported {}%", importer.percent_done() * 100.0);
    }

    Ok(())
}
