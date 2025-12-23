use shazamio_core::{Recognizer, SearchParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let recognizer = Recognizer::new(Some(10));

    let params = SearchParams::new(Some(12));

    let signature = recognizer
        .recognize_path("sample.wav".to_string(), Some(params))
        .await?;

    println!("Timestamp: {}", signature.timestamp);
    println!("Timezone: {}", signature.timezone);
    println!("Samples: {}", signature.signature.samples);

    Ok(())
}
