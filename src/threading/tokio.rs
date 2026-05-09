/// await all until any error occurs, or all succeed
#[macro_export]
macro_rules! tokio_when_all {
    ($futures:expr) => {
        tokio::try_join!($($futures),*)
    };
}