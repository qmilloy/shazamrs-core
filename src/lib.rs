//! # shazamrs-core
//!
//! Audio fingerprinting and Shazam signature generation, extracted as the
//! low-level engine behind the `shazamrs` crate.
//!
//! Given a decodable audio file or raw byte buffer, this crate:
//! 1. Decodes and downsamples the audio to 16 kHz mono PCM (via `rodio`,
//!    falling back to an `ffmpeg` subprocess for unsupported formats).
//! 2. Runs it through a short-time FFT to extract time/frequency peaks,
//!    producing a [`Signature`] compatible with Shazam's discovery API.
//!
//! The public surface is intentionally small: construct a [`Recognizer`],
//! then call [`Recognizer::recognize_path`] or [`Recognizer::recognize_bytes`].
//! Everything else (the FFT/peak-picking algorithm, the binary signature
//! encoding, and the `ffmpeg` fallback) is an internal implementation detail.

#![warn(missing_docs)]

mod errors;
mod fingerprinting;
mod params;
mod response;
mod utils;

pub use crate::errors::SignatureError;
pub use crate::params::SearchParams;
pub use crate::response::{Geolocation, Signature, SignatureSong};
use crate::utils::{convert_signature, unwrap_decoded_signature};
use fingerprinting::algorithm::SignatureGenerator;
use log::{debug, info};

/// Generates Shazam-compatible audio signatures from files or raw bytes.
///
/// A `Recognizer` is cheap to construct and clone; it only holds the
/// default segment duration used when a call doesn't supply its own
/// [`SearchParams`].
#[derive(Clone)]
pub struct Recognizer {
    segment_duration_seconds: u32,
}

impl Recognizer {
    /// Create a new `Recognizer`.
    ///
    /// `segment_duration_seconds` controls how many seconds of audio are
    /// sampled (from the middle of the track) when generating a signature;
    /// it defaults to 10 seconds when `None`. Individual calls to
    /// [`Recognizer::recognize_path`] or [`Recognizer::recognize_bytes`] can
    /// override this default by passing their own [`SearchParams`].
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

    /// Generate a signature from raw audio bytes.
    ///
    /// `value` must contain a complete, decodable audio file (e.g. the
    /// contents of a `.mp3`/`.wav`/`.flac`/`.ogg` file read into memory).
    /// `options` overrides the segment duration this `Recognizer` was
    /// created with, if provided.
    ///
    /// # Errors
    ///
    /// Returns [`SignatureError`] if the audio can't be decoded or the
    /// resulting signature can't be encoded.
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

    /// Generate a signature from an audio file on disk.
    ///
    /// Supported formats depend on `rodio`, with an `ffmpeg` subprocess
    /// used as a fallback for formats `rodio` can't decode (e.g. `.wma`,
    /// `.m4a`) when `ffmpeg` is available on the system. `options`
    /// overrides the segment duration this `Recognizer` was created with,
    /// if provided.
    ///
    /// # Errors
    ///
    /// Returns [`SignatureError`] if the file can't be read/decoded or the
    /// resulting signature can't be encoded.
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
