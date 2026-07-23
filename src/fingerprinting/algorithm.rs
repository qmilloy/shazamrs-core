use crate::fingerprinting::ffmpeg_wrapper::{decode_with_ffmpeg, decode_with_ffmpeg_from_bytes};
use crate::fingerprinting::hanning::HANNING_WINDOW_2048_MULTIPLIERS;
use crate::fingerprinting::signature_format::{DecodedSignature, FrequencyBand, FrequencyPeak};
use chfft::RFft1D;
use std::collections::HashMap;
use std::error::Error;
use std::io::{BufReader, Cursor};

/// Streaming FFT/peak-picking engine that turns 16 kHz mono PCM audio into
/// a [`DecodedSignature`] (a Shazam-compatible fingerprint).
///
/// This is a fairly direct port of Shazam's own signature algorithm: audio
/// is processed in 128-sample steps through a 2048-sample sliding window
/// (with a Hanning window applied), FFT'd, and scanned for local-maximum
/// frequency peaks across both the frequency and time domains. Peaks are
/// bucketed into one of four frequency bands (250–520 Hz, 520–1450 Hz,
/// 1450–3500 Hz, 3500–5500 Hz) to build the final fingerprint.
///
/// Use one of the `make_signature_from_*` associated functions rather than
/// constructing this type directly.
pub struct SignatureGenerator {
    ring_buffer_of_samples: Vec<i16>,
    reordered_ring_buffer_of_samples: Vec<f32>,
    fft_outputs: Vec<Vec<f32>>,
    spread_fft_outputs: Vec<Vec<f32>>,
    ring_buffer_of_samples_index: usize,
    fft_outputs_index: usize,
    fft_object: RFft1D<f32>,
    spread_fft_outputs_index: usize,
    num_spread_ffts_done: u32,
    signature: DecodedSignature,
}

impl SignatureGenerator {
    /// Decode an in-memory audio buffer and generate its signature.
    ///
    /// Tries `rodio` first, falling back to an `ffmpeg` subprocess (via
    /// [`decode_with_ffmpeg_from_bytes`]) if `rodio` can't decode the
    /// format. The decoded audio is downsampled to 16 kHz mono, and only a
    /// `segment_duration_seconds`-long slice (default 10s) taken from the
    /// middle of the track is fingerprinted.
    pub fn make_signature_from_bytes(bytes: Vec<u8>, segment_duration_seconds: Option<u32>) -> Result<DecodedSignature, Box<dyn Error>> {
        // Create a cursor around the byte array for decoding
        let cursor = Cursor::new(bytes.clone());

        let decoder = rodio::Decoder::new(cursor).or_else(|_decoding_error| {
            // Use the original bytes vector here
            decode_with_ffmpeg_from_bytes(&bytes)
        })?;

        // Convert the decoded samples to 16 kHz mono PCM, similar to make_signature_from_file
        // Here, we use UniformSourceIterator from rodio to downsample and convert to mono if necessary
        let converted_file = rodio::source::UniformSourceIterator::new(decoder, 1, 16000);
        let raw_pcm_samples: Vec<i16> = converted_file.collect();

        // Process the PCM samples as in make_signature_from_buffer
        let duration_seconds = segment_duration_seconds.unwrap_or(10);
        let sample_rate = 16000;
        let segment_samples = (duration_seconds * sample_rate) as usize;

        let raw_pcm_samples_slice: &[i16] = if raw_pcm_samples.len() > segment_samples {
            let middle = raw_pcm_samples.len() / 2;
            let half_segment = segment_samples / 2;
            if middle >= half_segment && middle + half_segment <= raw_pcm_samples.len() {
                &raw_pcm_samples[middle - half_segment..middle + half_segment]
            } else {
                &raw_pcm_samples[..segment_samples]
            }
        } else {
            &raw_pcm_samples[..]
        };

        // Generate signature from buffer
        let signature =
            SignatureGenerator::make_signature_from_buffer(raw_pcm_samples_slice.to_vec());

        // Return the generated signature
        Ok(signature)
    }
    /// Decode an audio file from disk and generate its signature.
    ///
    /// Tries `rodio` first, falling back to an `ffmpeg` subprocess (via
    /// [`decode_with_ffmpeg`]) if `rodio` can't decode the format (e.g.
    /// `.wma`, `.m4a`). The decoded audio is downsampled to 16 kHz mono,
    /// and only a `segment_duration_seconds`-long slice (default 10s)
    /// taken from the middle of the track is fingerprinted.
    pub fn make_signature_from_file(file_path: &str, segment_duration_seconds: Option<u32>) -> Result<DecodedSignature, Box<dyn Error>> {
        // Decode the .WAV, .MP3, .OGG or .FLAC file

        let mut decoder = rodio::Decoder::new(BufReader::new(std::fs::File::open(file_path)?));

        if let Err(ref _decoding_error) = decoder {
            // Try to decode with FFMpeg, if available, in case of failure with
            // Rodio (most likely due to the use of a format unsupported by
            // Rodio, such as .WMA or .MP4/.AAC)

            if let Some(new_decoder) = decode_with_ffmpeg(file_path) {
                decoder = Ok(new_decoder);
            }
        }

        // Downsample the raw PCM samples to 16 KHz, and skip to the middle of the file
        // in order to increase recognition odds. Take N (10 default) seconds of sample.
        let duration_seconds = segment_duration_seconds.unwrap_or(10);
        let sample_rate = 16000;
        let segment_samples = (duration_seconds * sample_rate) as usize;

        let converted_file = rodio::source::UniformSourceIterator::new(decoder?, 1, 16000);
        let raw_pcm_samples: Vec<i16> = converted_file.collect();
        let slice_len = raw_pcm_samples.len().min(segment_samples);
        let mut raw_pcm_samples_slice: &[i16] = &raw_pcm_samples[..slice_len];

        if raw_pcm_samples.len() > segment_samples {
            let middle = raw_pcm_samples.len() / 2;
            raw_pcm_samples_slice = &raw_pcm_samples[middle - segment_samples/2 .. middle + segment_samples/2];
        }

        let res = SignatureGenerator::make_signature_from_buffer(raw_pcm_samples_slice.to_vec());
        Ok(res)
    }

    /// Run the FFT/peak-picking algorithm directly over already-decoded
    /// signed 16-bit mono PCM samples at a 16 kHz sample rate.
    ///
    /// This is the core routine that [`Self::make_signature_from_file`] and
    /// [`Self::make_signature_from_bytes`] both delegate to once decoding
    /// and downsampling are done.
    pub fn make_signature_from_buffer(s16_mono_16khz_buffer: Vec<i16>) -> DecodedSignature {
        let mut this = SignatureGenerator {
            ring_buffer_of_samples: vec![0i16; 2048],
            ring_buffer_of_samples_index: 0,

            reordered_ring_buffer_of_samples: vec![0.0f32; 2048],

            fft_outputs: vec![vec![0.0f32; 1025]; 256],
            fft_outputs_index: 0,

            fft_object: RFft1D::new(2048),

            spread_fft_outputs: vec![vec![0.0f32; 1025]; 256],
            spread_fft_outputs_index: 0,

            num_spread_ffts_done: 0,

            signature: DecodedSignature {
                sample_rate_hz: 16000,
                number_samples: s16_mono_16khz_buffer.len() as u32,
                frequency_band_to_sound_peaks: HashMap::new(),
            },
        };
        for chunk in s16_mono_16khz_buffer.chunks_exact(128) {
            this.do_fft(chunk);

            this.do_peak_spreading();

            this.num_spread_ffts_done += 1;

            if this.num_spread_ffts_done >= 46 {
                this.do_peak_recognition();
            }
        }

        this.signature
    }

    fn do_fft(&mut self, s16_mono_16khz_buffer: &[i16]) {
        // Copy the 128 input s16le samples to the local ring buffer

        self.ring_buffer_of_samples
            [self.ring_buffer_of_samples_index..self.ring_buffer_of_samples_index + 128]
            .copy_from_slice(s16_mono_16khz_buffer);

        self.ring_buffer_of_samples_index += 128;
        self.ring_buffer_of_samples_index &= 2047;

        // Reorder the items (put the latest data at end) and apply Hanning window

        for (index, multiplier) in HANNING_WINDOW_2048_MULTIPLIERS.iter().enumerate() {
            self.reordered_ring_buffer_of_samples[index] = self.ring_buffer_of_samples
                [(index + self.ring_buffer_of_samples_index) & 2047]
                as f32
                * multiplier;
        }

        // Perform Fast Fourier transform
        let reordered_slice: &[f32] = self.reordered_ring_buffer_of_samples.as_ref();

        let complex_fft_results = self.fft_object.forward(reordered_slice);

        assert_eq!(complex_fft_results.len(), 1025);

        // Turn complex into reals, and put the results into a local array

        let real_fft_results = &mut self.fft_outputs[self.fft_outputs_index];

        for index in 0..=1024 {
            real_fft_results[index] = ((complex_fft_results[index].re.powi(2)
                + complex_fft_results[index].im.powi(2))
                / ((1 << 17) as f32))
                .max(0.0000000001);
        }

        self.fft_outputs_index += 1;
        self.fft_outputs_index &= 255;
    }

    fn do_peak_spreading(&mut self) {
        let real_fft_results =
            &self.fft_outputs[((self.fft_outputs_index as i32 - 1) & 255) as usize];

        let spread_fft_results = &mut self.spread_fft_outputs[self.spread_fft_outputs_index];

        // Perform frequency-domain spreading of peak values
        spread_fft_results.copy_from_slice(real_fft_results);

        for position in 0..=1022 {
            spread_fft_results[position] = spread_fft_results[position]
                .max(spread_fft_results[position + 1])
                .max(spread_fft_results[position + 2]);
        }

        let spread_fft_results_copy = spread_fft_results.clone();

        for position in 0..=1024 {
            for former_fft_number in &[1, 3, 6] {
                let former_fft_output = &mut self.spread_fft_outputs
                    [((self.spread_fft_outputs_index as i32 - *former_fft_number) & 255) as usize];

                former_fft_output[position] =
                    former_fft_output[position].max(spread_fft_results_copy[position]);
            }
        }

        self.spread_fft_outputs_index += 1;
        self.spread_fft_outputs_index &= 255;
    }

    fn do_peak_recognition(&mut self) {
        // Note: when substracting an array index, casting to signed is needed
        // to avoid underflow panics at runtime.

        let fft_minus_46 = &self.fft_outputs[((self.fft_outputs_index as i32 - 46) & 255) as usize];
        let fft_minus_49 =
            &self.spread_fft_outputs[((self.spread_fft_outputs_index as i32 - 49) & 255) as usize];

        for bin_position in 10..=1014 {
            // Ensure that the bin is large enough to be a peak

            if fft_minus_46[bin_position] >= 1.0 / 64.0
                && fft_minus_46[bin_position] >= fft_minus_49[bin_position - 1]
            {
                // Ensure that it is frequency-domain local minimum

                let mut max_neighbor_in_fft_minus_49: f32 = 0.0;

                for neighbor_offset in &[-10, -7, -4, -3, 1, 2, 5, 8] {
                    max_neighbor_in_fft_minus_49 = max_neighbor_in_fft_minus_49
                        .max(fft_minus_49[(bin_position as i32 + *neighbor_offset) as usize]);
                }

                if fft_minus_46[bin_position] > max_neighbor_in_fft_minus_49 {
                    // Ensure that it is a time-domain local minimum

                    let mut max_neighbor_in_other_adjacent_ffts = max_neighbor_in_fft_minus_49;

                    for other_offset in &[
                        -53, -45, 165, 172, 179, 186, 193, 200, 214, 221, 228, 235, 242, 249,
                    ] {
                        let other_fft = &self.spread_fft_outputs[((self.spread_fft_outputs_index
                            as i32
                            + other_offset)
                            & 255)
                            as usize];

                        max_neighbor_in_other_adjacent_ffts =
                            max_neighbor_in_other_adjacent_ffts.max(other_fft[bin_position - 1]);
                    }

                    if fft_minus_46[bin_position] > max_neighbor_in_other_adjacent_ffts {
                        // This is a peak, store the peak

                        let fft_pass_number = self.num_spread_ffts_done - 46;

                        let peak_magnitude: f32 =
                            fft_minus_46[bin_position].ln().max(1.0 / 64.0) * 1477.3 + 6144.0;
                        let peak_magnitude_before: f32 =
                            fft_minus_46[bin_position - 1].ln().max(1.0 / 64.0) * 1477.3 + 6144.0;
                        let peak_magnitude_after: f32 =
                            fft_minus_46[bin_position + 1].ln().max(1.0 / 64.0) * 1477.3 + 6144.0;

                        let peak_variation_1: f32 =
                            peak_magnitude * 2.0 - peak_magnitude_before - peak_magnitude_after;
                        let peak_variation_2: f32 = (peak_magnitude_after - peak_magnitude_before)
                            * 32.0
                            / peak_variation_1;

                        let corrected_peak_frequency_bin: u16 = (
                            (bin_position as i32 * 64) + (peak_variation_2 as i32)
                        ) as u16;

                        assert!(peak_variation_1 >= 0.0);

                        // Convert back a FFT bin to a frequency, given a 16 KHz sample
                        // rate, 1024 useful bins and the multiplication by 64 made before
                        // storing the information

                        let frequency_hz: f32 =
                            corrected_peak_frequency_bin as f32 * (16000.0 / 2.0 / 1024.0 / 64.0);

                        // Ignore peaks outside the 250 Hz-5.5 KHz range, store them into
                        // a lookup table that will be used to generate the binary fingerprint
                        // otherwise

                        let frequency_band = match frequency_hz as i32 {
                            250..=519 => FrequencyBand::_250_520,
                            520..=1449 => FrequencyBand::_520_1450,
                            1450..=3499 => FrequencyBand::_1450_3500,
                            3500..=5500 => FrequencyBand::_3500_5500,
                            _ => {
                                continue;
                            }
                        };

                        self.signature.frequency_band_to_sound_peaks
                            .entry(frequency_band)
                            .or_default();

                        self.signature
                            .frequency_band_to_sound_peaks
                            .get_mut(&frequency_band)
                            .unwrap()
                            .push(FrequencyPeak {
                                fft_pass_number,
                                peak_magnitude: peak_magnitude as u16,
                                corrected_peak_frequency_bin,
                            });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn sine_wave_samples(
        frequency_hz: f32,
        sample_rate_hz: u32,
        duration_seconds: f32,
        amplitude: i16,
    ) -> Vec<i16> {
        let num_samples = (sample_rate_hz as f32 * duration_seconds) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate_hz as f32;
                (amplitude as f32 * (2.0 * PI * frequency_hz * t).sin()) as i16
            })
            .collect()
    }

    #[test]
    fn detects_peaks_in_expected_frequency_band_for_pure_tone() {
        // A 1 kHz tone falls within the 520 Hz-1450 Hz band.
        let samples = sine_wave_samples(1000.0, 16000, 2.0, 12000);

        let signature = SignatureGenerator::make_signature_from_buffer(samples);

        let peaks = signature
            .frequency_band_to_sound_peaks
            .get(&FrequencyBand::_520_1450)
            .expect("expected peaks in the 520-1450 Hz band");
        assert!(!peaks.is_empty());
    }

    #[test]
    fn silence_produces_no_peaks() {
        let samples = vec![0i16; 16000 * 2];

        let signature = SignatureGenerator::make_signature_from_buffer(samples);

        let total_peaks: usize = signature
            .frequency_band_to_sound_peaks
            .values()
            .map(|peaks| peaks.len())
            .sum();
        assert_eq!(total_peaks, 0);
    }
}
