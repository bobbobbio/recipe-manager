use crate::database;
use crate::database::models::{
    Ingredient, IngredientId, IngredientMeasurement, IngredientUsageId, RecipeCategoryId,
    RecipeDuration, RecipeHandle, RecipeId,
};
use diesel::ExpressionMethods as _;
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

pub fn delete_category(conn: &mut database::Connection, delete_id: RecipeCategoryId) -> bool {
    let count: i64 = {
        use database::schema::recipes::dsl::*;

        recipes
            .filter(category.eq(delete_id))
            .count()
            .get_result(conn)
            .unwrap()
    };

    if count == 0 {
        use database::schema::recipe_categories::dsl::*;
        use diesel::delete;

        delete(recipe_categories.filter(id.eq(delete_id)))
            .execute(conn)
            .unwrap();
        true
    } else {
        false
    }
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
    use database::schema::recipes::dsl::*;
    use diesel::delete;

    delete(recipes.filter(id.eq(delete_id)))
        .execute(conn)
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
