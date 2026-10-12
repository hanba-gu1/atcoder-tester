#[derive(thiserror::Error, Debug)]
#[error("{}", self.0)]
pub struct UserError(String);
impl UserError {
    pub fn new(message: String) -> Self {
        Self(message)
    }
}

#[macro_export]
macro_rules! user_error {
    ($($f:tt)*) => {
        $crate::error::UserError::new(format!($($f)*))
    };
}
#[macro_export]
macro_rules! fail {
    ($($f:tt)*) => {
        ::std::result::Result::Err($crate::user_error!($($f)*))?
    };
}
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $($f:tt)*) => {
        if !$cond {
            $crate::fail!($($f)*)
        }
    };
}
