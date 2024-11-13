// @generated automatically by Diesel CLI.

diesel::table! {
    ingredient_usages (id) {
        id -> Integer,
        recipe_id -> Integer,
        ingredient_id -> Integer,
        quantity -> Float,
        quantity_units -> Nullable<crate::database::models::IngredientMeasurementMapping>,
    }
}

diesel::table! {
    ingredients (id) {
        id -> Integer,
        name -> Text,
        category -> Nullable<Text>,
    }
}

diesel::table! {
    recipe_categories (id) {
        id -> Integer,
        name -> Text,
    }
}

diesel::table! {
    recipes (id) {
        id -> Integer,
        name -> Text,
        description -> Text,
        duration -> crate::database::models::RecipeDurationMapping,
        category -> Integer,
    }
}

diesel::joinable!(ingredient_usages -> ingredients (ingredient_id));
diesel::joinable!(ingredient_usages -> recipes (recipe_id));
diesel::joinable!(recipes -> recipe_categories (category));

diesel::allow_tables_to_appear_in_same_query!(
    ingredient_usages,
    ingredients,
    recipe_categories,
    recipes,
);
