use std::fmt;

use serde::de::{self, DeserializeOwned, Deserializer, IgnoredAny, MapAccess, Visitor};

use crate::http_context::HttpContext;

use super::{
    body_reader::{content_type_media, read_body_for_binding},
    error::BindError,
};

/// Binds **one field** of the request body.
///
/// The field name is the action argument name, like `FromQuery<T>`. Use [`Body`](super::Body)
/// to bind the whole body into a DTO (or to read it as raw text).
///
/// Supported `Content-Type` values (JSON is assumed when it is missing):
/// - `application/json`, `text/json`: a member of the top-level JSON object.
/// - `application/xml`, `text/xml`: a child element of the root element.
///
/// `T` is any serde `Deserialize` type. A missing field is HTTP 422, unless `T` is an `Option`
/// (then it is `None`, so is `null`), that is [`FromOptionBody<T>`]. A wrong type, a body that is not
/// an object or a Content-Type mismatch is HTTP 422 too.
///
/// # Warning
///
/// - For a repeated scalar field, each occurrence must parse as `T`. The last value is returned.
/// - Each field binder parses the buffered document again. For many fields prefer `Body<Dto>`.
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

impl<T> FromBody<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for FromBody<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FromBody<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
