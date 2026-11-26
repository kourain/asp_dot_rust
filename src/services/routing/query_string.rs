use std::collections::HashMap;

/// Percent-decode one query component. `+` is a space (form encoding), `%2B` stays a plus.
/// `None` when the decoded bytes are not valid UTF-8.
pub(crate) fn decode_query_component(raw: &str) -> Option<String> {
    if raw.contains('+') {
        urlencoding::decode(&raw.replace('+', " ")).ok().map(std::borrow::Cow::into_owned)
    } else {
        urlencoding::decode(raw).ok().map(std::borrow::Cow::into_owned)
    }
}

/// Decoded nonempty keys and their raw values, in wire order.
pub(crate) fn query_pairs(query: &str) -> impl DoubleEndedIterator<Item = (String, &str)> {
    query.split('&').filter_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_query_component(key).filter(|key| !key.is_empty())?;
        Some((key, value))
    })
}

/// Split a query string (or a form-urlencoded body) into decoded `(valid, invalid)` parameter maps.
///
/// - Empty pairs (`a=1&&b=2`) and pairs with an empty key (`=v`) are skipped.
/// - A key without `=` (`?debug`) has an empty value, so it can be used as a flag.
/// - A repeated key keeps its last occurrence, whether that occurrence is valid or not.
/// - A value that is not valid UTF-8 after decoding goes to `invalid` with its raw text,
///   so the binder can report it instead of seeing a missing or altered value.
/// - A key that is not valid UTF-8 cannot equal an action parameter name and is dropped.
pub(crate) fn parse_query_string(query: &str) -> (HashMap<String, String>, HashMap<String, String>) {
    let mut valid = HashMap::new();
    let mut invalid = HashMap::new();
    for (key, raw_value) in query_pairs(query) {
        match decode_query_component(raw_value) {
            Some(value) => {
                invalid.remove(&key);
                valid.insert(key, value);
            }
            None => {
                valid.remove(&key);
                invalid.insert(key, raw_value.to_string());
            }
        }
    }
    (valid, invalid)
}
