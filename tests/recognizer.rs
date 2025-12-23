use shazamio_core::Recognizer;

#[tokio::test]
async fn recognizer_fails_on_invalid_path() {
    let recognizer = Recognizer::new(None);

    let result = recognizer
        .recognize_path("this_file_does_not_exist.wav".to_string(), None)
        .await;

    assert!(result.is_err());
}
