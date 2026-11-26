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
            None => T::deserialize(de::value::UnitDeserializer::<de::value::Error>::new())
                .map(Self)
                .map_err(|_| BindError::MissingField(name.to_owned())),
        }
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
