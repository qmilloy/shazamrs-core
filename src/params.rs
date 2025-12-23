use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchParams {
    pub segment_duration_seconds: u32,
}

impl SearchParams {
    pub fn new(segment_duration_seconds: Option<u32>) -> Self {
        Self {
            segment_duration_seconds: segment_duration_seconds.unwrap_or(10),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_segment_duration_is_10() {
        let params = SearchParams::new(None);
        assert_eq!(params.segment_duration_seconds, 10);
    }

    #[test]
    fn custom_segment_duration_is_used() {
        let params = SearchParams::new(Some(15));
        assert_eq!(params.segment_duration_seconds, 15);
    }
}
