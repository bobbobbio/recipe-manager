// Copyright 2023 Remi Bernotavicius

use plist::dictionary::Dictionary;
use plist::{Uid, Value};
use std::fmt;
use std::path::Path;

#[derive(Debug)]
enum DecodeError {
    InvalidUid(Uid),
    NoSuchKey {
        needle: String,
        haystack: Vec<String>,
    },
    WrongType {
        expected: &'static str,
        actual: &'static str,
    },
    Utf8(std::str::Utf8Error),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUid(u) => write!(f, "decode error: invalid UID {u:?}"),
            Self::NoSuchKey { needle, haystack } => write!(
                f,
                "decode error: no such key {needle:?} found in {haystack:?}"
            ),
            Self::WrongType { expected, actual } => write!(
                f,
                "decode error: expected type {expected:?} but found type {actual:?}"
            ),
            Self::Utf8(e) => write!(f, "decode error: {e}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl From<std::str::Utf8Error> for DecodeError {
    fn from(e: std::str::Utf8Error) -> Self {
        Self::Utf8(e)
    }
}

type Result<T> = std::result::Result<T, DecodeError>;

trait DictionaryExt {
    fn get_or_error(&self, key: &str) -> Result<&Value>;
    fn get_dictionary_or_error(&self, key: &str) -> Result<&Dictionary>;
    fn get_array_or_error(&self, key: &str) -> Result<&Vec<Value>>;
    fn get_data_or_error(&self, key: &str) -> Result<&[u8]>;
    fn get_string_or_error(&self, key: &str) -> Result<&str>;
    #[expect(dead_code)]
    fn get_signed_integer_or_error(&self, key: &str) -> Result<i64>;
    #[expect(dead_code)]
    fn get_unsigned_integer_or_error(&self, key: &str) -> Result<u64>;
    fn get_real_or_error(&self, key: &str) -> Result<f64>;
}

impl DictionaryExt for Dictionary {
    fn get_or_error(&self, key: &str) -> Result<&Value> {
        self.get(key).ok_or(DecodeError::NoSuchKey {
            needle: key.into(),
            haystack: self.keys().cloned().collect(),
        })
    }

    fn get_dictionary_or_error(&self, key: &str) -> Result<&Dictionary> {
        self.get_or_error(key)?.as_dictionary_or_error()
    }

    fn get_array_or_error(&self, key: &str) -> Result<&Vec<Value>> {
        self.get_or_error(key)?.as_array_or_error()
    }

    fn get_data_or_error(&self, key: &str) -> Result<&[u8]> {
        self.get_or_error(key)?.as_data_or_error()
    }

    fn get_string_or_error(&self, key: &str) -> Result<&str> {
        self.get_or_error(key)?.as_string_or_error()
    }

    fn get_signed_integer_or_error(&self, key: &str) -> Result<i64> {
        self.get_or_error(key)?.as_signed_integer_or_error()
    }

    fn get_unsigned_integer_or_error(&self, key: &str) -> Result<u64> {
        self.get_or_error(key)?.as_unsigned_integer_or_error()
    }

    fn get_real_or_error(&self, key: &str) -> Result<f64> {
        self.get_or_error(key)?.as_real_or_error()
    }
}

trait ValueExt {
    fn as_dictionary_or_error(&self) -> Result<&Dictionary>;
    fn as_array_or_error(&self) -> Result<&Vec<Value>>;
    fn as_data_or_error(&self) -> Result<&[u8]>;
    fn as_string_or_error(&self) -> Result<&str>;
    fn as_signed_integer_or_error(&self) -> Result<i64>;
    fn as_unsigned_integer_or_error(&self) -> Result<u64>;
    fn as_real_or_error(&self) -> Result<f64>;

    #[expect(dead_code)]
    fn into_dictionary_or_error(self) -> Result<Dictionary>;
    #[expect(dead_code)]
    fn into_array_or_error(self) -> Result<Vec<Value>>;
    #[expect(dead_code)]
    fn into_data_or_error(self) -> Result<Vec<u8>>;
    fn into_string_or_error(self) -> Result<String>;

    fn type_str(&self) -> &'static str;
}

impl ValueExt for Value {
    fn as_dictionary_or_error(&self) -> Result<&Dictionary> {
        let actual = self.type_str();
        self.as_dictionary().ok_or(DecodeError::WrongType {
            expected: "Dictionary",
            actual,
        })
    }

    fn as_array_or_error(&self) -> Result<&Vec<Value>> {
        let actual = self.type_str();
        self.as_array().ok_or(DecodeError::WrongType {
            expected: "Array",
            actual,
        })
    }

    fn as_data_or_error(&self) -> Result<&[u8]> {
        let actual = self.type_str();
        self.as_data().ok_or(DecodeError::WrongType {
            expected: "Data",
            actual,
        })
    }

    fn as_string_or_error(&self) -> Result<&str> {
        let actual = self.type_str();
        self.as_string().ok_or(DecodeError::WrongType {
            expected: "String",
            actual,
        })
    }

    fn as_signed_integer_or_error(&self) -> Result<i64> {
        let actual = self.type_str();
        self.as_signed_integer().ok_or(DecodeError::WrongType {
            expected: "signed integer",
            actual,
        })
    }

    fn as_unsigned_integer_or_error(&self) -> Result<u64> {
        let actual = self.type_str();
        self.as_unsigned_integer().ok_or(DecodeError::WrongType {
            expected: "unsigned integer",
            actual,
        })
    }

    fn as_real_or_error(&self) -> Result<f64> {
        let actual = self.type_str();
        self.as_real().ok_or(DecodeError::WrongType {
            expected: "Real",
            actual,
        })
    }

    fn into_dictionary_or_error(self) -> Result<Dictionary> {
        let actual = self.type_str();
        self.into_dictionary().ok_or(DecodeError::WrongType {
            expected: "Dictionary",
            actual,
        })
    }

    fn into_array_or_error(self) -> Result<Vec<Value>> {
        let actual = self.type_str();
        self.into_array().ok_or(DecodeError::WrongType {
            expected: "Array",
            actual,
        })
    }

    fn into_data_or_error(self) -> Result<Vec<u8>> {
        let actual = self.type_str();
        self.into_data().ok_or(DecodeError::WrongType {
            expected: "Data",
            actual,
        })
    }

    fn into_string_or_error(self) -> Result<String> {
        let actual = self.type_str();
        self.into_string().ok_or(DecodeError::WrongType {
            expected: "String",
            actual,
        })
    }

    fn type_str(&self) -> &'static str {
        match self {
            Self::Array(_) => "Array",
            Self::Dictionary(_) => "Dictionary",
            Self::Boolean(_) => "Boolean",
            Self::Data(_) => "Data",
            Self::Date(_) => "Date",
            Self::Real(_) => "Real",
            Self::Integer(_) => "Integer",
            Self::String(_) => "String",
            Self::Uid(_) => "Uid",
            _ => "Unknown",
        }
    }
}

trait ArrayExt {
    fn iter_as_dictionary_or_error(&self) -> Result<std::vec::IntoIter<&Dictionary>>;
}

impl ArrayExt for Vec<Value> {
    fn iter_as_dictionary_or_error(&self) -> Result<std::vec::IntoIter<&Dictionary>> {
        let refs: Vec<&Dictionary> = self
            .iter()
            .map(|i| i.as_dictionary_or_error())
            .collect::<Result<_>>()?;
        Ok(refs.into_iter())
    }
}

struct ArchiverDataDecoder<'a> {
    objects: &'a [Value],
}

impl<'a> ArchiverDataDecoder<'a> {
    fn new(objects: &'a [Value]) -> Self {
        Self { objects }
    }

    fn decode_array(&self, array: &[Value]) -> Result<Vec<Value>> {
        let mut new_array = vec![];
        for obj in array {
            new_array.push(self.decode_value(obj)?);
        }
        Ok(new_array)
    }

    fn decode_ns_mutable_dictionary(&self, fields: &Dictionary) -> Result<Dictionary> {
        let keys = fields.get_array_or_error("NS.keys")?;
        let values = fields.get_array_or_error("NS.objects")?;

        let mut dict = Dictionary::new();
        for (key, value) in keys.iter().zip(values) {
            let key = self.decode_value(key)?.into_string_or_error()?;
            let value = self.decode_value(value)?;
            dict.insert(key, value);
        }
        Ok(dict)
    }

    fn decode_ns_mutable_string(&self, fields: &Dictionary) -> Result<String> {
        Ok(fields.get_string_or_error("NS.string")?.to_string())
    }

    fn decode_ns_date(&self, fields: &Dictionary) -> Result<f64> {
        fields.get_real_or_error("NS.time")
    }

    fn decode_ns_mutable_array(&self, fields: &Dictionary) -> Result<Vec<Value>> {
        let values = fields.get_array_or_error("NS.objects")?;
        values.iter().map(|v| self.decode_value(v)).collect()
    }

    fn decode_ns_mutable_data(&self, fields: &Dictionary) -> Result<Vec<u8>> {
        Ok(fields.get_data_or_error("NS.data")?.to_vec())
    }

    fn decode_unknown_object(&self, class_name: &str, fields: &Dictionary) -> Result<Dictionary> {
        assert!(
            !class_name.starts_with("NS"),
            "unknown {class_name}: {fields:#?}"
        );
        let mut obj = fields.clone();
        obj.insert("$class".into(), class_name.into());
        self.decode_dictionary(&obj)
    }

    fn decode_object(&self, class: &Value, fields: &Dictionary) -> Result<Value> {
        let class = self.decode_value(class)?;
        let class_name = class.as_dictionary_or_error()?.get_or_error("$classname")?;
        let class_name = class_name.as_string_or_error()?;

        Ok(match class_name {
            "NSDate" => self.decode_ns_date(fields)?.into(),
            "NSMutableArray" => self.decode_ns_mutable_array(fields)?.into(),
            "NSMutableData" => Value::Data(self.decode_ns_mutable_data(fields)?),
            "NSMutableDictionary" => self.decode_ns_mutable_dictionary(fields)?.into(),
            "NSMutableString" => self.decode_ns_mutable_string(fields)?.into(),
            _ => self.decode_unknown_object(class_name, fields)?.into(),
        })
    }

    fn decode_dictionary(&self, dict: &Dictionary) -> Result<Dictionary> {
        let mut new_dict = Dictionary::new();
        for (key, value) in dict {
            new_dict.insert(key.clone(), self.decode_value(value)?);
        }

        Ok(new_dict)
    }

    fn decode_uid(&self, id: &Uid) -> Result<Value> {
        let index = usize::try_from(id.get()).map_err(|_| DecodeError::InvalidUid(*id))?;
        if index >= self.objects.len() {
            return Err(DecodeError::InvalidUid(*id));
        }

        let object = &self.objects[index];
        self.decode_value(object)
    }

    fn decode_value(&self, object: &Value) -> Result<Value> {
        use Value::*;
        Ok(match object {
            Array(v) => self.decode_array(v)?.into(),
            Dictionary(v) => {
                if let Some(c) = v.get("$class") {
                    self.decode_object(c, v)?
                } else {
                    self.decode_dictionary(v)?.into()
                }
            }
            Uid(v) => self.decode_uid(v)?,
            o => o.clone(),
        })
    }
}

fn decode(file: &Value) -> Result<Value> {
    let file = file.as_dictionary_or_error()?;
    let top = file.get_dictionary_or_error("$top")?;
    let root = top.get_or_error("root")?;
    let objects = file.get_array_or_error("$objects")?;
    let decoder = ArchiverDataDecoder::new(objects);
    decoder.decode_value(root)
}

#[derive(Debug)]
pub struct Ingredient {
    pub name: String,
    pub category: String,
    pub quantity: f64,
    pub measurement: String,
}

#[derive(Debug)]
pub struct Recipe {
    pub name: String,
    pub other: String,
    pub time: String,
    pub ingredients: Vec<Ingredient>,
}

#[derive(Debug)]
pub struct RecipeBox {
    pub name: String,
    pub recipes: Vec<Recipe>,
}

fn decode_recipes(root: &Value) -> Result<Vec<RecipeBox>> {
    let mut recipe_boxes_out = vec![];

    let root = root.as_dictionary_or_error()?;
    let recipe_boxes = root.get_array_or_error("recipeBoxes")?;
    for b in recipe_boxes.iter_as_dictionary_or_error()? {
        let mut recipes_out = vec![];

        let properties = b.get_dictionary_or_error("properties")?;
        let name = properties.get_string_or_error("Name")?;
        let recipies = b.get_array_or_error("recipes")?;
        for r in recipies.iter_as_dictionary_or_error()? {
            let properties = r.get_dictionary_or_error("properties")?;
            let ingredients = r.get_array_or_error("ingredients")?;

            let name = properties.get_string_or_error("Name")?;
            let other = properties.get_data_or_error("Other")?;
            let other_str = std::str::from_utf8(other)?;
            let time = properties.get_string_or_error("Time")?;
            let mut ingredients_out = vec![];
            for i in ingredients.iter_as_dictionary_or_error()? {
                let properties = i.get_dictionary_or_error("properties")?;
                let name = properties.get_string_or_error("Name")?;
                let category = properties.get_string_or_error("Catagory")?;

                let quantity_value = properties.get_or_error("Quantity")?;
                let quantity =
                    if let Ok(quantity_int) = quantity_value.as_unsigned_integer_or_error() {
                        quantity_int as f64
                    } else {
                        quantity_value.as_real_or_error()?
                    };

                let measurement = properties.get_string_or_error("Measurement")?;
                ingredients_out.push(Ingredient {
                    name: name.into(),
                    category: category.into(),
                    quantity,
                    measurement: measurement.into(),
                });
            }
            recipes_out.push(Recipe {
                name: name.into(),
                other: other_str.into(),
                time: time.into(),
                ingredients: ingredients_out,
            });
        }

        recipe_boxes_out.push(RecipeBox {
            name: name.into(),
            recipes: recipes_out,
        });
    }
    Ok(recipe_boxes_out)
}

pub fn decode_recipes_from_path(path: impl AsRef<Path>) -> crate::Result<Vec<RecipeBox>> {
    let contents = Value::from_file(path)?;
    let value = decode(&contents).unwrap();
    Ok(decode_recipes(&value)?)
}

pub fn decode_calendar_from_path(path: impl AsRef<Path>) -> crate::Result<()> {
    let contents = Value::from_file(path)?;
    let value = decode(&contents)?;
    println!("{value:#?}");
    Ok(())
}
