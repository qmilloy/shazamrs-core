use crate::errors::SignatureError;
use crate::fingerprinting::communication;
use crate::fingerprinting::communication::get_signature_json;
use crate::fingerprinting::signature_format::DecodedSignature;
use crate::response::{Geolocation, Signature, SignatureSong};

/// Convert an internal, wire-format [`communication::Signature`] into the
/// public [`Signature`] type returned by [`crate::Recognizer`].
pub fn convert_signature(signature: communication::Signature) -> Signature {
    Signature::new(
        Geolocation::new(
            signature.geolocation.altitude,
            signature.geolocation.latitude,
            signature.geolocation.longitude,
        ),
        SignatureSong::new(
            signature.signature.samples,
            signature.signature.timestamp,
            signature.signature.uri,
        ),
        signature.timestamp,
        signature.timezone,
    )
}

/// Finalize a [`DecodedSignature`] (raw FFT peak data) into a
/// [`communication::Signature`] ready to be encoded and sent to Shazam.
pub fn unwrap_decoded_signature(
    data: DecodedSignature,
) -> Result<communication::Signature, SignatureError> {
    get_signature_json(&data).map_err(|e| SignatureError::new(e.to_string()))
}
