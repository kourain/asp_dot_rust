macro_rules! some_or_error {
    ($option:expr, $target:expr, $message:expr) => {
        match $option {
            Some(value) => value,
            None => return $crate::utils::compiler_error::create_compiler_error($target, $message),
        }
    };
}

macro_rules! ok_or_error {
    ($result:expr, $target:expr, $message:expr) => {
        match $result {
            Ok(value) => value,
            Err(_) => return $crate::utils::compiler_error::create_compiler_error($target, $message),
        }
    };
}

pub(crate) use {ok_or_error, some_or_error};