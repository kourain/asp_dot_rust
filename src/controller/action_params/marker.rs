/// Marker for [`FromParamField`](super::FromParamField): the field must be present (`T`).
pub struct RequiredField;

/// Marker for [`FromParamField`](super::FromParamField): the field may be absent (`Option<T>`).
pub struct OptionalField;
