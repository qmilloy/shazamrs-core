mod errors;
mod fingerprinting;
mod params;
mod response;
mod utils;

pub use crate::errors::SignatureError;
pub use crate::params::SearchParams;
pub use crate::response::Signature;
use crate::utils::{convert_signature, unwrap_decoded_signature};
use fingerprinting::algorithm::SignatureGenerator;
use log::{debug, info};

#[derive(Clone)]
pub struct Recognizer {
    segment_duration_seconds: u32,
}

impl Recognizer {
    pub fn new(segment_duration_seconds: Option<u32>) -> Self {
        let duration = segment_duration_seconds.unwrap_or(10);
        info!(
            "Recognizer created with segment_duration_seconds = {}",
            duration
        );
        Self {
            segment_duration_seconds: duration,
        }
    }

    pub async fn recognize_bytes(
        &self,
        value: Vec<u8>,
        options: Option<SearchParams>,
    ) -> Result<Signature, SignatureError> {
        debug!("recognize_bytes called with {} bytes", value.len());

        let search_options =
            options.unwrap_or_else(|| SearchParams::new(Some(self.segment_duration_seconds)));

        let data = SignatureGenerator::make_signature_from_bytes(
            value,
            Some(search_options.segment_duration_seconds),
        )
        .map_err(|e| SignatureError::new(e.to_string()))?;

        let sig = unwrap_decoded_signature(data)?;
        Ok(convert_signature(sig))
    }

    pub async fn recognize_path(
        &self,
        value: String,
        options: Option<SearchParams>,
    ) -> Result<Signature, SignatureError> {
        debug!("recognize_path called with {}", value);

        let search_options =
            options.unwrap_or_else(|| SearchParams::new(Some(self.segment_duration_seconds)));

        let data = SignatureGenerator::make_signature_from_file(
            &value,
            Some(search_options.segment_duration_seconds),
        )
        .map_err(|e| SignatureError::new(e.to_string()))?;

        let sig = unwrap_decoded_signature(data)?;
        Ok(convert_signature(sig))
    }
}
