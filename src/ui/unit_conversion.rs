use crate::database::models::IngredientMeasurement;

#[derive(PartialEq, Eq, Debug)]
pub enum MeasurementKind {
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
            IngredientMeasurement::Quart => Self::Volume,
            IngredientMeasurement::Tablespoons => Self::Volume,
            IngredientMeasurement::Teaspoons => Self::Volume,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum MeasurementClass {
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
            IngredientMeasurement::Quart => Self::Us,
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
        IngredientMeasurement::Quart => 192.0,
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
        IngredientMeasurement::Quart => 946.353,
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

pub fn conversion_factor(a: IngredientMeasurement, b: IngredientMeasurement) -> f32 {
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
