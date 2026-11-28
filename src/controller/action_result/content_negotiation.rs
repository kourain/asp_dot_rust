use std::io;

use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{ACCEPT, CONTENT_TYPE, LOCATION},
};
use serde::Serialize;

use crate::http_context::{HttpResponse, append_vary};

#[derive(Clone, Copy)]
pub(super) enum Format {
    Json,
    Xml,
}

impl Format {
    fn media_type(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Xml => "application/xml",
        }
    }
}

// Commas inside a quoted media parameter belong to that parameter.
fn media_ranges(value: &str) -> impl Iterator<Item = &str> {
    let mut quoted = false;
    let mut escaped = false;
    value.split(move |ch| {
        if escaped {
            escaped = false;
        } else if quoted && ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            quoted = !quoted;
        }
        ch == ',' && !quoted
    })
}

// HTTP weights have up to three decimal places.
fn quality(value: &str) -> Option<u16> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let decimal = fraction.bytes().zip([100, 10, 1]).map(|(b, place)| u16::from(b - b'0') * place).sum::<u16>();
    match whole {
        "0" => Some(decimal),
        "1" if decimal == 0 => Some(1000),
        _ => None,
    }
}

fn parse_range(item: &str) -> Option<(&str, u16)> {
    let mut parts = item.split(';');
    let media = parts.next()?.trim();
    let mut weight = None;
    for param in parts {
        let (name, value) = param.trim().split_once('=')?;
        // The offered representations have no media parameters.
        if !name.trim().eq_ignore_ascii_case("q") || weight.is_some() {
            return None;
        }
        weight = Some(quality(value.trim())?);
    }
    Some((media, weight.unwrap_or(1000)))
}

fn select(headers: &HeaderMap) -> Option<Format> {
    if !headers.contains_key(ACCEPT) {
        return Some(Format::Json);
    }
    let formats = [Format::Json, Format::Xml];
    let mut matches = [None; 2];
    for (range, weight) in headers.get_all(ACCEPT).iter().filter_map(|value| value.to_str().ok()).flat_map(media_ranges).filter_map(parse_range) {
        for (format, best) in formats.iter().zip(&mut matches) {
            let specificity = if range.eq_ignore_ascii_case(format.media_type()) {
                2
            } else if range.eq_ignore_ascii_case("application/*") {
                1
            } else if range == "*/*" {
                0
            } else {
                continue;
            };
            *best = (*best).max(Some((specificity, weight)));
        }
    }
    // The most specific range determines each format's weight, including exclusions.
    // Prefer JSON when both representations match equally well.
    formats
        .into_iter()
        .zip(matches)
        .rev()
        .filter_map(|(format, best)| {
            let (specificity, weight) = best?;
            (weight > 0).then_some(((weight, specificity), format))
        })
        .max_by_key(|(rank, _)| *rank)
        .map(|(_, format)| format)
}

pub(super) fn write<T: Serialize>(body: &T, headers: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
    if response.has_started() {
        return Err(io::Error::other("content negotiation requires uncommitted response headers"));
    }
    append_vary(&mut response.headers, "Accept");
    let Some(format) = select(headers) else {
        response.status_code = StatusCode::NOT_ACCEPTABLE;
        response.headers.remove(CONTENT_TYPE);
        response.headers.remove(LOCATION);
        return Ok(());
    };
    write_format(format, body, response)
}

pub(super) fn write_format<T: Serialize>(format: Format, body: &T, response: &mut HttpResponse) -> io::Result<()> {
    if response.has_started() {
        return Err(io::Error::other("setting a serialized content type requires uncommitted response headers"));
    }
    #[derive(Serialize)]
    #[serde(rename = "response")]
    struct XmlDocument<'a, T> {
        value: &'a T,
    }
    let bytes = match format {
        Format::Json => serde_json::to_vec(body).map_err(io::Error::other),
        Format::Xml => quick_xml::se::to_string(&XmlDocument { value: body }).map(String::into_bytes).map_err(io::Error::other),
    };
    match bytes {
        Ok(bytes) => {
            response.body = bytes;
            response.headers.insert(CONTENT_TYPE, HeaderValue::from_static(format.media_type()));
            Ok(())
        }
        Err(error) => {
            response.headers.remove(LOCATION);
            response.headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
            Err(error)
        }
    }
}
