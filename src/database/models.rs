// Copyright 2023 Remi Bernotavicius

use derive_more::Display;
use diesel::associations::{Associations, Identifiable};
use diesel::deserialize::Queryable;
use diesel::expression::Selectable;
use diesel::prelude::Insertable;
use diesel_derive_enum::DbEnum;
use diesel_derive_newtype::DieselNewType;
use strum::EnumIter;

#[derive(DieselNewType, Debug, Hash, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
pub struct IngredientId(i32);

impl IngredientId {
    pub const INITIAL: Self = Self(1);

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(table_name = crate::database::schema::ingredients)]
pub struct Ingredient {
    pub id: IngredientId,
    pub name: String,
    pub category: Option<String>,
}

impl Ingredient {
    pub fn to_handle(&self) -> IngredientHandle {
        IngredientHandle {
            id: self.id,
            name: self.name.clone(),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(table_name = crate::database::schema::ingredients)]
pub struct IngredientHandle {
    pub id: IngredientId,
    pub name: String,
}

#[derive(Debug, Display, EnumIter, Hash, Copy, Clone, PartialEq, Eq, DbEnum)]
pub enum RecipeDuration {
    #[display("short")]
    Short,
    #[display("medium")]
    Medium,
    #[display("long")]
    Long,
    #[display("really long")]
    ReallyLong,
}

impl RecipeDuration {
    pub fn iter() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }
}

#[derive(DieselNewType, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct RecipeCategoryId(i32);

impl RecipeCategoryId {
    pub const INITIAL: Self = Self(1);

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(table_name = crate::database::schema::recipe_categories)]
pub struct RecipeCategory {
    pub id: RecipeCategoryId,
    pub name: String,
}

#[derive(DieselNewType, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct RecipeId(i32);

impl RecipeId {
    pub const INITIAL: Self = Self(1);

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Associations, Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(belongs_to(RecipeCategory, foreign_key = category))]
#[diesel(table_name = crate::database::schema::recipes)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: String,
    pub description: String,
    pub duration: RecipeDuration,
    pub category: RecipeCategoryId,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(table_name = crate::database::schema::recipes)]
pub struct RecipeHandle {
    pub id: RecipeId,
    pub name: String,
}

#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq, EnumIter, DbEnum, PartialOrd, Ord)]
pub enum IngredientMeasurement {
    Cups,
    FluidOunces,
    Grams,
    Kilograms,
    Kiloliters,
    Liters,
    Milligrams,
    Milliliters,
    Ounces,
    Pounds,
    Quart,
    Tablespoons,
    Teaspoons,
}

impl IngredientMeasurement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cups => "cups",
            Self::FluidOunces => "fl. oz.",
            Self::Grams => "g",
            Self::Kilograms => "kg",
            Self::Kiloliters => "kL",
            Self::Liters => "L",
            Self::Milligrams => "mg",
            Self::Milliliters => "mL",
            Self::Ounces => "oz.",
            Self::Pounds => "lbs.",
            Self::Quart => "qt.",
            Self::Tablespoons => "tbsp.",
            Self::Teaspoons => "tsp.",
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        <Self as strum::IntoEnumIterator>::iter()
    }
}

#[derive(DieselNewType, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct IngredientUsageId(i32);

impl IngredientUsageId {
    pub const INITIAL: Self = Self(1);

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Associations, Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(belongs_to(Recipe))]
#[diesel(belongs_to(Ingredient))]
#[diesel(primary_key(id))]
#[diesel(table_name = crate::database::schema::ingredient_usages)]
pub struct IngredientUsage {
    pub id: IngredientUsageId,
    pub recipe_id: RecipeId,
    pub ingredient_id: IngredientId,
    pub quantity: f32,
    pub quantity_units: Option<IngredientMeasurement>,
}

#[derive(DieselNewType, Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub struct IngredientCaloriesEntryId(i32);

impl IngredientCaloriesEntryId {
    pub const INITIAL: Self = Self(1);

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Associations, Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(belongs_to(Ingredient))]
#[diesel(primary_key(id))]
#[diesel(table_name = crate::database::schema::ingredient_calories)]
pub struct IngredientCaloriesEntry {
    pub id: IngredientCaloriesEntryId,
    pub ingredient_id: IngredientId,
    pub calories: f32,
    pub quantity: f32,
    pub quantity_units: Option<IngredientMeasurement>,
}

#[derive(Associations, Queryable, Selectable, Identifiable, Insertable, Clone)]
#[diesel(belongs_to(RecipeCategory, foreign_key = recipe_id))]
#[diesel(primary_key(day))]
#[diesel(table_name = crate::database::schema::calendar)]
pub struct CalendarEntry {
    pub day: chrono::NaiveDate,
    pub recipe_id: RecipeId,
}
