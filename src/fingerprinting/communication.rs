use std::error::Error;
use std::time::SystemTime;

use crate::fingerprinting::signature_format::DecodedSignature;

/// Placeholder geolocation, in the shape Shazam's discovery API expects.
#[derive(Debug)]
pub struct GeolocationResponse {
    pub(crate) altitude: i16,
    pub(crate) latitude: i8,
    pub(crate) longitude: i8,
}

/// Encoded fingerprint and sample metadata, in the shape Shazam's
/// discovery API expects.
#[derive(Debug)]
pub struct SignatureSong {
    pub(crate) samples: u32,
    pub(crate) timestamp: u32,
    pub(crate) uri: String,
}

/// The full request payload shape sent to Shazam's discovery API.
///
/// Converted into the crate's public [`crate::response::Signature`] by
/// [`crate::utils::convert_signature`].
#[derive(Debug)]
pub struct Signature {
    pub(crate) geolocation: GeolocationResponse,
    pub(crate) signature: SignatureSong,
    pub(crate) timestamp: u32,
    pub(crate) timezone: String,
}

/// Build a [`Signature`] request payload from a [`DecodedSignature`].
///
/// Encodes the fingerprint to its base64 URI form, stamps it with the
/// current time, and attaches a fixed placeholder geolocation/timezone
/// (this crate does not use the device's real location).
pub fn get_signature_json(signature: &DecodedSignature) -> Result<Signature, Box<dyn Error>> {
    let timestamp_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_millis();
    let samples =
        (signature.number_samples as f32 / signature.sample_rate_hz as f32 * 1000.) as u32;
    Ok(Signature {
        geolocation: GeolocationResponse {
            altitude: 300,
            latitude: 45,
            longitude: 2,
        },
        signature: SignatureSong {
            samples,
            timestamp: timestamp_ms as u32,
            uri: signature.encode_to_uri()?,
        },
        timestamp: timestamp_ms as u32,
        timezone: "Europe/Paris".to_string(),
    })
}
