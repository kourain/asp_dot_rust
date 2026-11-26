use std::{fmt, marker::PhantomData};

use serde::de::{self, DeserializeOwned, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, Visitor};

use crate::{http_context::HttpRequest, services::routing::query_string::parse_query_string};

use super::{
    body_reader::{content_type_media, read_body_bytes},
    error::BindError,
};

/// Read the whole body from its start and leave the cursor at the start again.
///
/// Several binders (`FromForm`, `FromBody`, `Form`, `Body`) can read the same body this way,
/// and the action can still read it itself.
pub(crate) async fn read_body_for_binding(request: &mut HttpRequest) -> Result<Vec<u8>, BindError> {
    request.body.enable_buffering().await.map_err(|e| BindError::BodyRead(e.to_string()))?;
    rewind(request)?;
    let bytes = read_body_bytes(request).await?;
    rewind(request)?;
    Ok(bytes)
}

fn rewind(request: &mut HttpRequest) -> Result<(), BindError> {
    request.body.seek(0).map_err(|e| BindError::BodyRead(e.to_string()))
}

/// A form binder accepts `application/x-www-form-urlencoded`, a missing or blank media type is accepted too.
pub(crate) fn check_form_media(request: &HttpRequest) -> Result<(), BindError> {
    let media = content_type_media(request).unwrap_or_default();
    if !media.is_empty() && media != "application/x-www-form-urlencoded" {
        return Err(BindError::UnsupportedMediaType(media));
    }
    Ok(())
}

/// Decoded value of form field `name`. The last occurrence wins. `+` is a space and `%XX` is decoded.
/// A key without `=` has an empty value.
pub(crate) fn form_field(bytes: &[u8], name: &str) -> Result<String, BindError> {
    let text = std::str::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?;
    let (fields, invalid) = parse_query_string(text);
    if let Some(raw) = invalid.get(name) {
        return Err(BindError::ParseField {
            name: name.to_string(),
            value: raw.clone(),
        });
    }
    fields.get(name).cloned().ok_or_else(|| BindError::MissingField(name.to_string()))
}

/// Field `name` of the top-level JSON object.
pub(crate) fn json_field<T: DeserializeOwned>(bytes: &[u8], name: &str) -> Result<T, BindError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let found = FieldSeed::<T>::new(name).deserialize(&mut deserializer).map_err(|e| BindError::Deserialize(e.to_string()))?;
    deserializer.end().map_err(|e| BindError::Deserialize(e.to_string()))?;
    found_or_missing(found, name)
}

/// Child element `name` of the XML root element.
pub(crate) fn xml_field<T: DeserializeOwned>(bytes: &[u8], name: &str) -> Result<T, BindError> {
    let mut deserializer = quick_xml::de::Deserializer::from_reader(bytes);
    let found = FieldSeed::<T>::new(name).deserialize(&mut deserializer).map_err(|e| BindError::Deserialize(e.to_string()))?;
    found_or_missing(found, name)
}

/// A missing field is an error unless `T` accepts "no value" (`Option<U>` is `None`).
fn found_or_missing<T: DeserializeOwned>(found: Option<T>, name: &str) -> Result<T, BindError> {
    match found {
        Some(value) => Ok(value),
        None => T::deserialize(de::value::UnitDeserializer::<de::value::Error>::new()).map_err(|_| BindError::MissingField(name.to_string())),
    }
}

/// Reads one named entry of a map and skips every other entry without building it.
/// Works for any serde format, so JSON objects and XML elements share it. The last occurrence of the key wins.
struct FieldSeed<'a, T> {
    name: &'a str,
    marker: PhantomData<fn() -> T>,
}

impl<'a, T> FieldSeed<'a, T> {
    fn new(name: &'a str) -> Self {
        Self { name, marker: PhantomData }
    }
}

impl<'de, T: de::Deserialize<'de>> DeserializeSeed<'de> for FieldSeed<'_, T> {
    type Value = Option<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de, T: de::Deserialize<'de>> Visitor<'de> for FieldSeed<'_, T> {
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut found = None;
        while let Some(key) = map.next_key::<String>()? {
            if key == self.name {
                found = Some(map.next_value::<T>().map_err(|e| <A::Error as de::Error>::custom(format!("field `{}`: {e}", self.name)))?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_field_reads_one_entry_and_skips_the_others() {
        let body = br#"{"a":{"deep":[1,2]},"name":"Ada","age":36,"tags":["x","y"]}"#;
        assert_eq!(json_field::<String>(body, "name").unwrap(), "Ada");
        assert_eq!(json_field::<u32>(body, "age").unwrap(), 36);
        assert_eq!(json_field::<Vec<String>>(body, "tags").unwrap(), ["x", "y"]);
    }

    #[test]
    fn json_missing_field_is_an_error_unless_the_type_is_optional() {
        let body = br#"{"name":"Ada"}"#;
        assert!(matches!(json_field::<u32>(body, "age"), Err(BindError::MissingField(name)) if name == "age"));
        assert_eq!(json_field::<Option<u32>>(body, "age").unwrap(), None);
        assert_eq!(json_field::<Option<String>>(br#"{"name":null}"#, "name").unwrap(), None);
    }

    #[test]
    fn json_wrong_type_and_bad_documents_are_errors() {
        let error = json_field::<u32>(br#"{"age":"x"}"#, "age").unwrap_err().to_string();
        assert!(error.contains("field `age`"), "{error}");
        assert!(matches!(json_field::<String>(b"[1]", "name"), Err(BindError::Deserialize(_))));
        assert!(matches!(json_field::<String>(b"", "name"), Err(BindError::Deserialize(_))));
        assert!(matches!(json_field::<String>(br#"{"name":"a"} x"#, "name"), Err(BindError::Deserialize(_))));
    }

    #[test]
    fn json_repeated_field_keeps_the_last_value() {
        assert_eq!(json_field::<u8>(br#"{"n":1,"n":2}"#, "n").unwrap(), 2);
    }

    #[test]
    fn xml_field_reads_a_child_element_of_the_root() {
        let body = b"<root><term>abc</term><age>36</age><flag>true</flag></root>";
        assert_eq!(xml_field::<String>(body, "term").unwrap(), "abc");
        assert_eq!(xml_field::<u32>(body, "age").unwrap(), 36);
        assert_eq!(xml_field::<bool>(body, "flag").unwrap(), true);
        assert!(matches!(xml_field::<String>(body, "nope"), Err(BindError::MissingField(_))));
        assert_eq!(xml_field::<Option<String>>(body, "nope").unwrap(), None);
    }

    #[test]
    fn form_field_decodes_and_reports_problems() {
        assert_eq!(form_field(b"a=1&b=x+y%2B", "b").unwrap(), "x y+");
        assert_eq!(form_field(b"flag&a=1", "flag").unwrap(), "");
        assert_eq!(form_field(b"a=1&a=2", "a").unwrap(), "2");
        assert!(matches!(form_field(b"a=1", "b"), Err(BindError::MissingField(_))));
        assert!(matches!(form_field(b"a=%FF", "a"), Err(BindError::ParseField { .. })));
        assert!(matches!(form_field(&[b'a', b'=', 0xFF], "a"), Err(BindError::InvalidUtf8)));
    }
}
