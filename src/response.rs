use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Geolocation {
    pub altitude: i16,
    pub latitude: i8,
    pub longitude: i8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignatureSong {
    pub samples: u32,
    pub timestamp: u32,
    pub uri: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Signature {
    pub geolocation: Geolocation,
    pub signature: SignatureSong,
    pub timestamp: u32,
    pub timezone: String,
}

impl Geolocation {
    pub fn new(altitude: i16, latitude: i8, longitude: i8) -> Self {
        Self {
            altitude,
            latitude,
            longitude,
        }
    }
}

impl SignatureSong {
    pub fn new(samples: u32, timestamp: u32, uri: String) -> Self {
        Self {
            samples,
            timestamp,
            uri,
        }
    }
}

impl Signature {
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
