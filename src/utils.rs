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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn convert_signature_maps_all_fields() {
        let comm_signature = communication::Signature {
            geolocation: communication::GeolocationResponse {
                altitude: 300,
                latitude: 45,
                longitude: 2,
            },
            signature: communication::SignatureSong {
                samples: 10000,
                timestamp: 1700000000,
                uri: "data:audio/vnd.shazam.sig;base64,AA==".to_string(),
            },
            timestamp: 1700000000,
            timezone: "Europe/Paris".to_string(),
        };

        let signature = convert_signature(comm_signature);

        assert_eq!(signature.geolocation.altitude, 300);
        assert_eq!(signature.geolocation.latitude, 45);
        assert_eq!(signature.geolocation.longitude, 2);
        assert_eq!(signature.signature.samples, 10000);
        assert_eq!(
            signature.signature.uri,
            "data:audio/vnd.shazam.sig;base64,AA=="
        );
        assert_eq!(signature.timestamp, 1700000000);
        assert_eq!(signature.timezone, "Europe/Paris");
    }

    #[test]
    fn unwrap_decoded_signature_builds_valid_uri_and_sample_duration() {
        let decoded = DecodedSignature {
            sample_rate_hz: 16000,
            number_samples: 32000,
            frequency_band_to_sound_peaks: HashMap::new(),
        };

        let comm_signature = unwrap_decoded_signature(decoded).unwrap();

        assert!(
            comm_signature
                .signature
                .uri
                .starts_with("data:audio/vnd.shazam.sig;base64,")
        );
        // 32000 samples at 16 kHz is 2000 ms.
        assert_eq!(comm_signature.signature.samples, 2000);
    }
}
