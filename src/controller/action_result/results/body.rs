//! Buffered representations for controller results.

use std::{
    any::TypeId,
    fmt,
    io::{self, Write},
};

use http::{HeaderMap, HeaderValue, header::CONTENT_TYPE};
use serde::Serialize;

use super::{
    HttpResponse,
    content_negotiation::{self, Format},
};

pub trait ResponseBody {
    fn write(&self, request_headers: &HeaderMap, response: &mut HttpResponse) -> io::Result<()>;
}

// Serde's Formatter serializer writes strings verbatim into the response buffer.
struct PlainText<'a, T>(&'a T);

impl<T: Serialize> fmt::Display for PlainText<'_, T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.serialize(formatter)
    }
}

impl<T: Serialize> ResponseBody for T {
    fn write(&self, headers: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        // Compare concrete types without imposing 'static on borrowed strings or DTOs.
        let ty = typeid::of::<T>();
        if ![TypeId::of::<String>(), TypeId::of::<&str>(), TypeId::of::<&String>(), TypeId::of::<&&str>()].contains(&ty) {
            return content_negotiation::write(self, headers, response);
        }
        if !response.has_started() {
            response.headers.entry(CONTENT_TYPE).or_insert(HeaderValue::from_static("text/plain"));
        }
        write!(&mut response.body, "{}", PlainText(self))
    }
}

/// Writes binary data with Content-Type application/octet-stream.
#[derive(Debug)]
pub struct Raw<T>(pub T);

/// Serializes the value as JSON regardless of the request's Accept header.
#[derive(Debug)]
pub struct Json<T>(pub T);

/// Serializes the value as XML regardless of the request's Accept header.
#[derive(Debug)]
pub struct Xml<T>(pub T);

fn write_binary(bytes: &[u8], response: &mut HttpResponse) -> io::Result<()> {
    if !response.has_started() {
        response.headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
    }
    response.body.extend_from_slice(bytes);
    Ok(())
}

macro_rules! binary_bodies {
    ($($ty:ty),+ $(,)?) => { $(
        impl ResponseBody for Raw<$ty> {
            fn write(&self, _: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
                write_binary(&self.0, response)
            }
        }
    )+ };
}
binary_bodies!(Vec<u8>, &[u8], hyper::body::Bytes);

impl<const N: usize> ResponseBody for Raw<[u8; N]> {
    fn write(&self, _: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        write_binary(&self.0, response)
    }
}

impl<const N: usize> ResponseBody for Raw<&[u8; N]> {
    fn write(&self, _: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        write_binary(self.0, response)
    }
}

impl<T: Serialize> ResponseBody for Json<T> {
    fn write(&self, _: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        content_negotiation::write_format(Format::Json, &self.0, response)
    }
}

impl<T: Serialize> ResponseBody for Xml<T> {
    fn write(&self, _: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        content_negotiation::write_format(Format::Xml, &self.0, response)
    }
}
