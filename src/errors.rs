use std::fmt;
use thiserror::Error;

/// Error returned when signature generation or decoding fails.
#[derive(Debug, Error, Clone)]
pub struct SignatureError {
    message: String,
}

impl SignatureError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_error_displays_message() {
        let err = SignatureError::new("boom".to_string());
        assert_eq!(err.to_string(), "boom");
    }
}
