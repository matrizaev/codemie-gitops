use std::error::Error;
use std::path::PathBuf;

use schemars::schema::RootSchema;
use typify::{TypeSpace, TypeSpaceSettings};

const SCHEMA_PATH: &str = "contracts/declaration-v1alpha1.schema.json";

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={SCHEMA_PATH}");
    let raw = std::fs::read_to_string(SCHEMA_PATH)?;
    let mut schema: serde_json::Value = serde_json::from_str(&raw)?;
    remove_unsupported_conditionals(&mut schema);
    disambiguate_camel_case_properties(&mut schema);
    let schema: RootSchema = serde_json::from_value(schema)?;

    let settings = TypeSpaceSettings::default();
    let mut type_space = TypeSpace::new(&settings);
    type_space.add_root_schema(schema)?;

    let output = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?)
        .join("declaration_types.rs");
    let generated = type_space
        .to_stream()
        .to_string()
        .replace("\"indexTypeCamel\"", "\"indexType\"");
    std::fs::write(output, generated)?;
    Ok(())
}

fn disambiguate_camel_case_properties(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::Object(properties)) = object.get_mut("properties")
                && let Some(index_type) = properties.remove("indexType")
            {
                properties.insert("indexTypeCamel".to_owned(), index_type);
            }
            if let Some(serde_json::Value::Array(required)) = object.get_mut("required") {
                for field in required {
                    if field.as_str() == Some("indexType") {
                        *field = serde_json::Value::String("indexTypeCamel".to_owned());
                    }
                }
            }
            object
                .values_mut()
                .for_each(disambiguate_camel_case_properties);
        }
        serde_json::Value::Array(values) => {
            values
                .iter_mut()
                .for_each(disambiguate_camel_case_properties);
        }
        _ => {}
    }
}

fn remove_unsupported_conditionals(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.remove("if");
            object.remove("then");
            object.remove("else");
            object.remove("not");
            object
                .values_mut()
                .for_each(remove_unsupported_conditionals);
        }
        serde_json::Value::Array(values) => {
            values.iter_mut().for_each(remove_unsupported_conditionals);
        }
        _ => {}
    }
}
