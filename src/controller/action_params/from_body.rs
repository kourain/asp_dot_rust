use std::fmt;

use serde::de::{self, DeserializeOwned, Deserializer, IgnoredAny, MapAccess, Visitor};

use crate::http_context::HttpContext;

use super::{
    body_reader::{content_type_media, read_body_for_binding},
    error::BindError,
};

/// Binds the top-level JSON member or XML child element named after the action argument.
/// A missing Content-Type defaults to JSON. Invalid data or an unsupported media type returns HTTP 422.
/// [`FromOptionBody<T>`] accepts a missing field or JSON `null`.
///
/// Each occurrence of a repeated field must parse as `T`. The last value wins.
/// Each field binder parses the document again. Use [`Body<Dto>`](super::Body) to bind several fields in one pass.
pub struct FromBody<T>(pub T);

/// An optional body field, an alias of `FromBody<Option<T>>`: a missing field or `null` is `None`.
///
/// It is a type alias, so it cannot be used as a constructor or a pattern, write `FromBody(Some(1))` instead.
pub type FromOptionBody<T> = FromBody<Option<T>>;

impl<T: DeserializeOwned> FromBody<T> {
    /// Bind body field `name`.
    pub async fn bind(name: &str, http_context: &mut HttpContext) -> Result<Self, BindError> {
        let media = content_type_media(&http_context.request).unwrap_or_else(|| "application/json".to_string());
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let mut value = None;
        let field = NamedField { name, value: &mut value };
        match media.as_str() {
            "application/json" | "text/json" => {
                let mut json = serde_json::Deserializer::from_slice(bytes);
                json.deserialize_map(field).and_then(|()| json.end()).map_err(|e| BindError::Deserialize(e.to_string()))?;
            }
            "application/xml" | "text/xml" => {
                quick_xml::de::Deserializer::from_reader(bytes)
                    .deserialize_map(field)
                    .map_err(|e| BindError::Deserialize(e.to_string()))?;
            }
            _ => return Err(BindError::UnsupportedMediaType(media)),
        }
        match value {
            Some(value) => Ok(Self(value)),
            None => T::deserialize(MissingField)
                .map(Self)
                .map_err(|_| BindError::MissingField(name.to_owned())),
        }
    }
}

// Follow Serde's missing-field convention: only a request to deserialize an option receives None.
// An explicit null still goes through the document's normal deserializer.
struct MissingField;

impl<'de> Deserializer<'de> for MissingField {
    type Error = de::value::Error;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
        Err(de::Error::custom("missing body field"))
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_none()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

/// Deserialize the named member in its original format, skipping unrelated values of any type.
struct NamedField<'a, T> {
    name: &'a str,
    value: &'a mut Option<T>,
}

impl<'de, T: de::Deserialize<'de>> Visitor<'de> for NamedField<'_, T> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == self.name {
                *self.value = Some(map.next_value().map_err(|e| de::Error::custom(format!("field `{}`: {e}", self.name)))?);
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(())
    }
}
