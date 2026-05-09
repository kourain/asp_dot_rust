pub trait Str {
    fn from_str(s: &str) -> Option<Self>
    where
        Self: Sized;
    fn as_str(&self) -> &str;
}