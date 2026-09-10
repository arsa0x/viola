use crate::{error::NativeError, native::Value};

pub fn get_str<'a>(obj: &'a Value, key: &str) -> Result<&'a str, NativeError> {
    match obj.get_field(key) {
        Some(Value::Str(s)) => Ok(s.as_ref()),
        Some(other) => Err(field_type_error(key, "str", other)),
        None => Err(missing_field_error(key)),
    }
}

pub fn get_str_opt<'a>(obj: &'a Value, key: &str) -> Result<Option<&'a str>, NativeError> {
    match obj.get_field(key) {
        Some(Value::Str(s)) => Ok(Some(s.as_ref())),
        Some(Value::Nil) | None => Ok(None),
        Some(other) => Err(field_type_error(key, "str", other)),
    }
}

pub fn get_bool_opt(obj: &Value, key: &str, default: bool) -> Result<bool, NativeError> {
    match obj.get_field(key) {
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::Nil) | None => Ok(default),
        Some(other) => Err(field_type_error(key, "bool", other)),
    }
}

pub fn get_array<'a>(obj: &'a Value, key: &str) -> Result<&'a [Value], NativeError> {
    match obj.get_field(key) {
        Some(Value::Array(items)) => Ok(items.as_slice()),
        Some(other) => Err(field_type_error(key, "array", other)),
        None => Err(missing_field_error(key)),
    }
}

pub fn get_array_opt<'a>(obj: &'a Value, key: &str) -> Result<&'a [Value], NativeError> {
    match obj.get_field(key) {
        Some(Value::Array(items)) => Ok(items.as_slice()),
        Some(Value::Nil) | None => Ok(&[]),
        Some(other) => Err(field_type_error(key, "array", other)),
    }
}

fn missing_field_error(key: &str) -> NativeError {
    NativeError::invalid_arg(format!("missing required field `{key}`"))
}

fn field_type_error(key: &str, expected: &str, got: &Value) -> NativeError {
    NativeError::invalid_arg(format!(
        "field `{key}` should be {expected}, got {}",
        got.type_name()
    ))
}
