/// Main library error type.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// SystemTime error.
    #[error(transparent)]
    SystemTime(#[from] std::time::SystemTimeError),

    /// Protobuf decode error.
    #[error(transparent)]
    ProstDecode(#[from] prost::DecodeError),

    #[error("crc mismatch, file: {file_crc} computed: {computed_crc}")]
    LenCrcMismatch { file_crc: u32, computed_crc: u32 },

    #[error("crc mismatch, file: {file_crc} computed: {computed_crc}")]
    CrcMismatch { file_crc: u32, computed_crc: u32 },

    /// Arbitrary errors wrapping.
    #[error(transparent)]
    Wrapped(Box<dyn std::error::Error + Send + Sync>),

    /// User generated error message, typically created via `bail!`.
    #[error("{0}")]
    Msg(String),
}

#[macro_export]
macro_rules! bail {
    ($msg:literal $(,)?) => {
        return Err($crate::Error::Msg(format!($msg).into()).bt())
    };
    ($err:expr $(,)?) => {
        return Err($crate::Error::Msg(format!($err).into()).bt())
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::Error::Msg(format!($fmt, $($arg)*).into()).bt())
    };
}

impl Error {
    pub fn wrap(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Wrapped(Box::new(err))
    }

    pub fn msg(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Msg(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
