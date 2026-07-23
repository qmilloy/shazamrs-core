use base64::engine::general_purpose;
use base64::Engine;
use byteorder::{LittleEndian, WriteBytesExt};
use crc32fast::Hasher;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::error::Error;
use std::io::{Cursor, Seek, SeekFrom, Write};

const DATA_URI_PREFIX: &str = "data:audio/vnd.shazam.sig;base64,";

/// A single detected frequency peak, as produced by
/// [`crate::fingerprinting::algorithm::SignatureGenerator`].
pub struct FrequencyPeak {
    /// Index of the FFT pass (46-pass window) in which this peak was
    /// detected, relative to the start of the signal.
    pub fft_pass_number: u32,
    /// Log-scaled magnitude of the peak.
    pub peak_magnitude: u16,
    /// FFT bin (frequency, corrected and rescaled) at which the peak occurred.
    pub corrected_peak_frequency_bin: u16,
}

/// The four frequency ranges Shazam's fingerprint format buckets peaks
/// into (250–520 Hz, 520–1450 Hz, 1450–3500 Hz, 3500–5500 Hz).
#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub enum FrequencyBand {
    /// 250 Hz – 520 Hz.
    _250_520 = 0,
    /// 520 Hz – 1450 Hz.
    _520_1450 = 1,
    /// 1450 Hz – 3500 Hz.
    _1450_3500 = 2,
    /// 3500 Hz – 5500 Hz.
    _3500_5500 = 3,
}

impl Ord for FrequencyBand {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as i32).cmp(&(*other as i32))
    }
}

impl PartialOrd for FrequencyBand {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some((*self as i32).cmp(&(*other as i32)))
    }
}

/// The in-memory result of fingerprinting an audio clip: sample metadata
/// plus every detected frequency peak, grouped by [`FrequencyBand`].
///
/// Produced by [`crate::fingerprinting::algorithm::SignatureGenerator`];
/// encode it to Shazam's wire format with [`Self::encode_to_binary`] or
/// [`Self::encode_to_uri`].
pub struct DecodedSignature {
    /// Sample rate of the audio that was fingerprinted, in Hz (always
    /// 16000 for signatures produced by this crate).
    pub sample_rate_hz: u32,
    /// Number of PCM samples that were fingerprinted.
    pub number_samples: u32,
    /// Detected peaks, grouped by frequency band.
    pub frequency_band_to_sound_peaks: HashMap<FrequencyBand, Vec<FrequencyPeak>>,
}

impl DecodedSignature {
    /// Encode this signature into Shazam's binary wire format.
    ///
    /// Produces the raw header + per-band peak data + CRC32 checksum
    /// layout that Shazam's discovery API expects, ready to be base64-encoded
    /// (see [`Self::encode_to_uri`]).
    pub fn encode_to_binary(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut cursor = Cursor::new(vec![]);

        // Please see the RawSignatureHeader structure definition above for
        // information about the following fields.

        cursor.write_u32::<LittleEndian>(0xcafe2580)?; // magic1
        cursor.write_u32::<LittleEndian>(0)?; // crc32 - Will write later
        cursor.write_u32::<LittleEndian>(0)?; // size_minus_header - Will write later
        cursor.write_u32::<LittleEndian>(0x94119c00)?; // magic2
        cursor.write_u32::<LittleEndian>(0)?; // void1
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(
            match self.sample_rate_hz {
                8000 => 1,
                11025 => 2,
                16000 => 3,
                32000 => 4,
                44100 => 5,
                48000 => 6,
                _ => {
                    panic!("Invalid sample rate passed when encoding Shazam packet");
                }
            } << 27,
        )?; // shifted_sample_rate_id
        cursor.write_u32::<LittleEndian>(0)?; // void2
        cursor.write_u32::<LittleEndian>(0)?;
        cursor.write_u32::<LittleEndian>(
            self.number_samples + (self.sample_rate_hz as f32 * 0.24) as u32,
        )?; // number_samples_plus_divided_sample_rate
        cursor.write_u32::<LittleEndian>((15 << 19) + 0x40000)?; // fixed_value

        cursor.write_u32::<LittleEndian>(0x40000000)?;
        cursor.write_u32::<LittleEndian>(0)?; // size_minus_header - Will write later

        let mut sorted_iterator: Vec<_> = self.frequency_band_to_sound_peaks.iter().collect();
        sorted_iterator.sort_by(|x, y| x.0.cmp(y.0));

        for (frequency_band, frequency_peaks) in sorted_iterator {
            let mut peaks_cursor = Cursor::new(vec![]);

            let mut fft_pass_number = 0;

            for frequency_peak in frequency_peaks {
                assert!(frequency_peak.fft_pass_number >= fft_pass_number);

                if frequency_peak.fft_pass_number - fft_pass_number >= 255 {
                    peaks_cursor.write_u8(0xff)?;
                    peaks_cursor.write_u32::<LittleEndian>(frequency_peak.fft_pass_number)?;

                    fft_pass_number = frequency_peak.fft_pass_number;
                }

                peaks_cursor.write_u8((frequency_peak.fft_pass_number - fft_pass_number) as u8)?;

                peaks_cursor.write_u16::<LittleEndian>(frequency_peak.peak_magnitude)?;
                peaks_cursor
                    .write_u16::<LittleEndian>(frequency_peak.corrected_peak_frequency_bin)?;

                fft_pass_number = frequency_peak.fft_pass_number;
            }

            let peaks_buffer = peaks_cursor.into_inner();

            cursor.write_u32::<LittleEndian>(0x60030040 + *frequency_band as u32)?;
            cursor.write_u32::<LittleEndian>(peaks_buffer.len() as u32)?;
            cursor.write_all(&peaks_buffer)?;
            for _padding_index in 0..((4 - peaks_buffer.len() as u32 % 4) % 4) {
                cursor.write_u8(0)?;
            }
        }

        let buffer_size = cursor.position() as u32;

        cursor.seek(SeekFrom::Start(8))?;
        cursor.write_u32::<LittleEndian>(buffer_size - 48)?;

        cursor.seek(SeekFrom::Start(48 + 4))?;
        cursor.write_u32::<LittleEndian>(buffer_size - 48)?;

        cursor.seek(SeekFrom::Start(4))?;
        let mut hasher = Hasher::new();
        hasher.update(&cursor.get_ref()[8..]);
        cursor.write_u32::<LittleEndian>(hasher.finalize())?; // crc32

        Ok(cursor.into_inner())
    }

    /// Encode this signature to the `data:audio/vnd.shazam.sig;base64,...`
    /// URI form used as the `uri` field of the request sent to Shazam.
    pub fn encode_to_uri(&self) -> Result<String, Box<dyn Error>> {
        Ok(format!(
            "{}{}",
            DATA_URI_PREFIX,
            general_purpose::STANDARD.encode(self.encode_to_binary()?)
        ))
    }

}
