use crate::database;
use crate::database::models::{
    Ingredient, IngredientCalories, IngredientCaloriesId, IngredientId, IngredientMeasurement,
    IngredientUsage, IngredientUsageId, Recipe, RecipeCategory, RecipeCategoryId, RecipeDuration,
    RecipeHandle, RecipeId,
};
use diesel::BoolExpressionMethods as _;
use diesel::Connection as _;
use diesel::ExpressionMethods as _;
use diesel::JoinOnDsl as _;
use diesel::QueryDsl as _;
use diesel::RunQueryDsl as _;
use diesel::SelectableHelper as _;

use std::collections::HashMap;

pub fn add_category(conn: &mut database::Connection, new_category_name: &str) {
    use database::schema::recipe_categories::dsl::*;
    use diesel::insert_into;

    insert_into(recipe_categories)
        .values(name.eq(new_category_name))
        .execute(conn)
        .unwrap();
}

pub fn add_ingredient_calories_entry(
    conn: &mut database::Connection,
    new_ingredient_id: IngredientId,
    new_calories: f32,
    new_quantity: f32,
    new_quantity_units: Option<IngredientMeasurement>,
) {
    use database::schema::ingredient_calories::dsl::*;
    use diesel::insert_into;

    insert_into(ingredient_calories)
        .values((
            ingredient_id.eq(new_ingredient_id),
            calories.eq(new_calories),
            quantity.eq(new_quantity),
            quantity_units.eq(new_quantity_units),
        ))
        .execute(conn)
        .unwrap();
}

pub fn delete_ingredient_calories_entry(
    conn: &mut database::Connection,
    delete_id: IngredientCaloriesId,
) {
    use database::schema::ingredient_calories::dsl::*;
    use diesel::delete;

    delete(ingredient_calories)
        .filter(id.eq(delete_id))
        .execute(conn)
        .unwrap();
}

pub fn delete_category(conn: &mut database::Connection, delete_id: RecipeCategoryId) -> bool {
    use database::schema::{recipe_categories, recipes};
    use diesel::delete;
    use diesel::dsl::{exists, not};

    let affected = delete(recipe_categories::table.filter(
        recipe_categories::id.eq(delete_id).and(not(exists(
            recipes::table.filter(recipes::category.eq(delete_id)),
        ))),
    ))
    .execute(conn)
    .unwrap();

    affected > 0
}

pub fn delete_ingredient(conn: &mut database::Connection, delete_id: IngredientId) -> bool {
    use database::schema::{ingredient_usages, ingredients};
    use diesel::delete;
    use diesel::dsl::{exists, not};

    let affected = delete(
        ingredients::table.filter(ingredients::id.eq(delete_id).and(not(exists(
            ingredient_usages::table.filter(ingredient_usages::ingredient_id.eq(delete_id)),
        )))),
    )
    .execute(conn)
    .unwrap();

    affected > 0
}

pub fn edit_category(
    conn: &mut database::Connection,
    id_to_edit: RecipeCategoryId,
    new_name: &str,
) {
    use database::schema::recipe_categories::dsl::*;
    use diesel::update;

    update(recipe_categories.filter(id.eq(id_to_edit)))
        .set(name.eq(new_name))
        .execute(conn)
        .unwrap();
}

pub fn delete_recipe(conn: &mut database::Connection, delete_id: RecipeId) {
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        use database::schema::{ingredient_usages, recipes};
        use diesel::delete;

        delete(ingredient_usages::table.filter(ingredient_usages::recipe_id.eq(delete_id)))
            .execute(conn)?;
        delete(recipes::table.filter(recipes::id.eq(delete_id))).execute(conn)?;
        Ok(())
    })
    .unwrap();
}

pub fn add_recipe(conn: &mut database::Connection, new_name: &str, new_category: RecipeCategoryId) {
    use database::schema::recipes::dsl::*;
    use diesel::insert_into;

    insert_into(recipes)
        .values((
            name.eq(new_name),
            description.eq(""),
            duration.eq(RecipeDuration::Short),
            category.eq(new_category),
        ))
        .execute(conn)
        .unwrap();
}

pub fn delete_recipe_ingredient(conn: &mut database::Connection, usage_id: IngredientUsageId) {
    use database::schema::ingredient_usages::dsl::*;
    use diesel::delete;

    delete(ingredient_usages)
        .filter(id.eq(usage_id))
        .execute(conn)
        .unwrap();
}

pub fn add_recipe_ingredient(
    conn: &mut database::Connection,
    new_recipe_id: RecipeId,
    new_ingredient_id: IngredientId,
    new_quantity: f32,
) {
    use database::schema::ingredient_usages::dsl::*;
    use diesel::insert_into;

    insert_into(ingredient_usages)
        .values((
            recipe_id.eq(new_recipe_id),
            ingredient_id.eq(new_ingredient_id),
            quantity.eq(new_quantity),
        ))
        .execute(conn)
        .unwrap();
}

pub fn edit_recipe_ingredient(
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

pub fn edit_recipe_duration(
    conn: &mut database::Connection,
    recipe_id: RecipeId,
    new_duration: RecipeDuration,
) {
    use database::schema::recipes::dsl::*;
    use diesel::update;

    update(recipes)
        .filter(id.eq(recipe_id))
        .set(duration.eq(new_duration))
        .execute(conn)
        .unwrap();
}

pub fn edit_recipe_category(
    conn: &mut database::Connection,
    recipe_id: RecipeId,
    new_category_id: RecipeCategoryId,
) {
    use database::schema::recipes::dsl::*;
    use diesel::update;

    update(recipes)
        .filter(id.eq(recipe_id))
        .set(category.eq(new_category_id))
        .execute(conn)
        .unwrap();
}

pub fn edit_recipe_description(
    conn: &mut database::Connection,
    recipe_id: RecipeId,
    new_description: &str,
) {
    use database::schema::recipes::dsl::*;
    use diesel::update;

    update(recipes)
        .filter(id.eq(recipe_id))
        .set(description.eq(new_description))
        .execute(conn)
        .unwrap();
}

pub fn edit_recipe_name(conn: &mut database::Connection, recipe_id: RecipeId, new_name: &str) {
    use database::schema::recipes::dsl::*;
    use diesel::update;

    update(recipes)
        .filter(id.eq(recipe_id))
        .set(name.eq(new_name))
        .execute(conn)
        .unwrap();
}

pub struct CachedQuery<IdT> {
    query: String,
    results: Vec<(IdT, String)>,
}

pub fn search_ingredients(
    conn: &mut database::Connection,
    cached_ingredient_search: &mut Option<CachedQuery<Ingredient>>,
    query: &str,
) -> Vec<(Ingredient, String)> {
    if let Some(cached) = cached_ingredient_search.as_ref() {
        if cached.query == query {
            return cached.results.clone();
        }
    }

    use database::schema::ingredients::dsl::*;
    use diesel::expression_methods::TextExpressionMethods as _;

    let result: Vec<_> = ingredients
        .select(Ingredient::as_select())
        .filter(name.like(format!("%{query}%")))
        .load(conn)
        .unwrap()
        .into_iter()
        .map(|i| (i.clone(), i.name))
        .collect();

    *cached_ingredient_search = Some(CachedQuery {
        query: query.into(),
        results: result.clone(),
    });
    result
}

pub fn get_calendar_week(
    conn: &mut database::Connection,
    start: chrono::NaiveWeek,
) -> HashMap<chrono::Weekday, RecipeHandle> {
    use chrono::Datelike as _;
    use database::schema::calendar::dsl::*;
    use diesel::BoolExpressionMethods as _;

    calendar
        .inner_join(database::schema::recipes::table)
        .select((day, RecipeHandle::as_select()))
        .filter(day.ge(start.first_day()).and(day.le(start.last_day())))
        .load(conn)
        .unwrap()
        .into_iter()
        .map(|(d, r): (chrono::NaiveDate, RecipeHandle)| (d.weekday(), r))
        .collect()
}

pub fn delete_calendar_entry(conn: &mut database::Connection, delete_day: chrono::NaiveDate) {
    use database::schema::calendar::dsl::*;
    use diesel::delete;

    delete(calendar.filter(day.eq(delete_day)))
        .execute(conn)
        .unwrap();
}

pub fn insert_or_update_calendar_entry(
    conn: &mut database::Connection,
    edit_date: chrono::NaiveDate,
    edit_recipe_id: RecipeId,
) {
    use database::schema::calendar::dsl::*;
    use diesel::insert_into;

    insert_into(calendar)
        .values((day.eq(edit_date), recipe_id.eq(edit_recipe_id)))
        .on_conflict(day)
        .do_update()
        .set(recipe_id.eq(edit_recipe_id))
        .execute(conn)
        .unwrap();
}

pub fn search_recipes(
    conn: &mut database::Connection,
    cached_recipe_search: &mut Option<CachedQuery<RecipeId>>,
    query: &str,
) -> Vec<(RecipeId, String)> {
    if let Some(cached) = cached_recipe_search.as_ref() {
        if cached.query == query {
            return cached.results.clone();
        }
    }

    use database::schema::recipes::dsl::*;
    use diesel::expression_methods::TextExpressionMethods as _;

    let result: Vec<_> = recipes
        .select(RecipeHandle::as_select())
        .filter(name.like(format!("%{query}%")))
        .load(conn)
        .unwrap()
        .into_iter()
        .map(|i| (i.id, i.name))
        .collect();

    *cached_recipe_search = Some(CachedQuery {
        query: query.into(),
        results: result.clone(),
    });
    result
}

pub fn add_ingredient(conn: &mut database::Connection, new_name: &str) {
    use database::schema::ingredients::dsl::*;
    use diesel::insert_into;

    insert_into(ingredients)
        .values(name.eq(new_name))
        .execute(conn)
        .unwrap();
}

pub fn search_ingredient_categories(
    conn: &mut database::Connection,
    cached_category_search: &mut Option<CachedQuery<()>>,
    query: &str,
) -> Vec<((), String)> {
    if let Some(cached) = cached_category_search.as_ref() {
        if cached.query == query {
            return cached.results.clone();
        }
    }

    use database::schema::ingredients::dsl::*;
    use diesel::expression_methods::TextExpressionMethods as _;

    let result: Vec<_> = ingredients
        .select(category)
        .filter(category.like(format!("%{query}%")))
        .distinct()
        .load(conn)
        .unwrap()
        .into_iter()
        .flat_map(|n: Option<String>| n.map(|n| ((), n)))
        .collect();

    *cached_category_search = Some(CachedQuery {
        query: query.into(),
        results: result.clone(),
    });
    result
}

pub fn update_ingredient(
    conn: &mut database::Connection,
    edit_id: IngredientId,
    edit_name: &str,
    edit_category: &str,
) {
    use database::schema::ingredients::dsl::*;
    use diesel::update;

    let edit_category = (!edit_category.is_empty()).then_some(edit_category);
    update(ingredients)
        .filter(id.eq(edit_id))
        .set((name.eq(edit_name), category.eq(edit_category)))
        .execute(conn)
        .unwrap();
}

pub fn search_recipes_by_ingredients(
    conn: &mut database::Connection,
    ingredient_ids: Vec<IngredientId>,
) -> Vec<RecipeHandle> {
    use database::schema::{ingredient_usages, ingredients, recipes};
    let mut query = recipes::table
        .inner_join(ingredient_usages::table.on(ingredient_usages::recipe_id.eq(recipes::id)))
        .inner_join(ingredients::table.on(ingredient_usages::ingredient_id.eq(ingredients::id)))
        .into_boxed();

    for i in ingredient_ids {
        query = query.or_filter(ingredients::id.eq(i));
    }

    query
        .select(RecipeHandle::as_select())
        .distinct()
        .load(conn)
        .unwrap()
}

pub fn get_ingredients_for_recipe(
    conn: &mut database::Connection,
    get_recipe_id: RecipeId,
) -> Vec<(IngredientUsage, Ingredient)> {
    use database::schema::{ingredient_usages, ingredients};

    ingredient_usages::table
        .filter(ingredient_usages::recipe_id.eq(get_recipe_id))
        .inner_join(ingredients::table)
        .select((IngredientUsage::as_select(), Ingredient::as_select()))
        .order_by(ingredients::name.asc())
        .load(conn)
        .unwrap()
}

pub fn get_ingredient_calories(
    conn: &mut database::Connection,
    get_ingredient_id: IngredientId,
) -> Vec<IngredientCalories> {
    use database::schema::ingredient_calories;

    ingredient_calories::table
        .filter(ingredient_calories::ingredient_id.eq(get_ingredient_id))
        .select(IngredientCalories::as_select())
        .load(conn)
        .unwrap()
}

pub fn get_recipe(
    conn: &mut database::Connection,
    recipe_id: RecipeId,
) -> (Recipe, String, Vec<(IngredientUsage, Ingredient)>) {
    use database::schema::{recipe_categories, recipes};

    let (recipe, category) = recipes::table
        .inner_join(recipe_categories::table)
        .filter(recipes::id.eq(recipe_id))
        .select((Recipe::as_select(), recipe_categories::name))
        .get_result(conn)
        .unwrap();
    let ingredients = get_ingredients_for_recipe(conn, recipe_id);
    (recipe, category, ingredients)
}

pub fn search_recipe_categories(
    conn: &mut database::Connection,
    cached_category_search: &mut Option<CachedQuery<RecipeCategoryId>>,
    query: &str,
) -> Vec<(RecipeCategoryId, String)> {
    if let Some(cached) = cached_category_search.as_ref() {
        if cached.query == query {
            return cached.results.clone();
        }
    }

    use database::schema::recipe_categories::dsl::*;
    use diesel::expression_methods::TextExpressionMethods as _;

    let result: Vec<_> = recipe_categories
        .select(RecipeCategory::as_select())
        .filter(name.like(format!("%{query}%")))
        .load(conn)
        .unwrap()
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();

    *cached_category_search = Some(CachedQuery {
        query: query.into(),
        results: result.clone(),
    });
    result
}
