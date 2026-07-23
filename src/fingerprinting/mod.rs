//! Internal audio-fingerprinting engine.
//!
//! Not part of the crate's public API (the module is private; see
//! [`crate::Recognizer`] for the supported entry points). Kept as separate
//! submodules for clarity:
//!
//! - [`algorithm`] — the FFT/peak-picking `SignatureGenerator`.
//! - [`signature_format`] — in-memory signature representation and its
//!   binary/base64 wire encoding.
//! - [`communication`] — the request payload shape sent to Shazam.
//! - [`ffmpeg_wrapper`] — fallback decoding via an `ffmpeg` subprocess for
//!   formats `rodio` can't handle.
//! - [`hanning`] — precomputed Hanning window coefficients used by the FFT.

pub mod algorithm;
pub mod communication;
pub mod ffmpeg_wrapper;
pub mod hanning;
pub mod signature_format;
