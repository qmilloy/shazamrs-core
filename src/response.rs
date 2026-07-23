use serde::{Deserialize, Serialize};

/// A placeholder geolocation attached to every generated [`Signature`].
///
/// Shazam's discovery API expects a location in each tag request; this
/// crate does not use the device's real location and instead sends a
/// fixed placeholder (see [`crate::fingerprinting::communication::get_signature_json`]).
#[derive(Clone, Serialize, Deserialize)]
pub struct Geolocation {
    /// Altitude in meters.
    pub altitude: i16,
    /// Latitude in degrees.
    pub latitude: i8,
    /// Longitude in degrees.
    pub longitude: i8,
}

/// The encoded audio fingerprint and sample metadata for a [`Signature`].
#[derive(Clone, Serialize, Deserialize)]
pub struct SignatureSong {
    /// Duration of the fingerprinted segment, in milliseconds.
    pub samples: u32,
    /// Unix timestamp (milliseconds) at which the signature was generated.
    pub timestamp: u32,
    /// The fingerprint itself, encoded as a `data:audio/vnd.shazam.sig;base64,...` URI.
    pub uri: String,
}

/// A complete, Shazam-compatible audio signature.
///
/// This is the payload sent to Shazam's discovery API to identify a track.
/// It is produced by [`crate::Recognizer::recognize_path`] or
/// [`crate::Recognizer::recognize_bytes`].
#[derive(Clone, Serialize, Deserialize)]
pub struct Signature {
    /// Placeholder location sent alongside the fingerprint.
    pub geolocation: Geolocation,
    /// The encoded audio fingerprint.
    pub signature: SignatureSong,
    /// Unix timestamp (milliseconds) at which the signature was generated.
    pub timestamp: u32,
    /// IANA timezone name sent alongside the fingerprint.
    pub timezone: String,
}

impl Geolocation {
    /// Create a new `Geolocation` from its raw components.
    pub fn new(altitude: i16, latitude: i8, longitude: i8) -> Self {
        Self {
            altitude,
            latitude,
            longitude,
        }
    }
}

impl SignatureSong {
    /// Create a new `SignatureSong` from its raw components.
    pub fn new(samples: u32, timestamp: u32, uri: String) -> Self {
        Self {
            samples,
            timestamp,
            uri,
        }
    }
}

impl Signature {
    /// Create a new `Signature` from its raw components.
    pub fn new(
        geolocation: Geolocation,
        signature: SignatureSong,
        timestamp: u32,
        timezone: String,
    ) -> Self {
        Self {
            geolocation,
            signature,
            timestamp,
            timezone,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_signature_structs() {
        let geo = Geolocation::new(100, 10, 20);
        let song = SignatureSong::new(1024, 123, "test://uri".to_string());
        let sig = Signature::new(geo, song, 456, "UTC".to_string());

        assert_eq!(sig.timestamp, 456);
        assert_eq!(sig.timezone, "UTC");
        assert_eq!(sig.signature.samples, 1024);
    }
}
