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

/// Repeated keys keep the last value, including a decoding failure.
pub(crate) fn parse_query_string(query: &str) -> HashMap<String, Result<String, String>> {
    query_pairs(query).map(|(key, raw)| {
        let value = decode_query_component(raw).ok_or_else(|| raw.to_owned());
        (key, value)
    }).collect()
}
