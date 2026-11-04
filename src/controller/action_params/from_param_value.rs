/// Parse a single path/query string value into `T`.
pub trait FromParamValue: Sized {
    fn from_param_value(value: &str) -> Result<Self, ()>;
}

impl FromParamValue for String {
    fn from_param_value(value: &str) -> Result<Self, ()> {
        Ok(value.to_string())
    }
}

macro_rules! impl_from_param_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FromParamValue for $ty {
                fn from_param_value(value: &str) -> Result<Self, ()> {
                    value.parse().map_err(|_| ())
                }
            }
        )*
    };
}

impl_from_param_value!(
    bool, char,
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64
);
