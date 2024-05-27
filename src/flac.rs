use claxon::FlacReader;
use flacenc::component::BitRepr;
use flacenc::error::Verify;
use hound::{WavReader, WavSpec, WavWriter};
use std::error::Error;
use std::fs;
use std::path::Path;
use tracing::{debug, error};

// Compress WAV data to FLAC format using flacenc crate
pub fn compress_flac(
    samples: &[i16],
    spec: &WavSpec,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    debug!("Compressing data into FLAC format...");

    let samples_as_i32: Vec<i32> = samples.iter().map(|&s| s as i32).collect();
    let (channels, bits_per_sample, sample_rate) =
        (spec.channels as u8, spec.bits_per_sample, spec.sample_rate);

    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let source = flacenc::source::MemSource::from_samples(
        &samples_as_i32,
        channels as usize,
        bits_per_sample as usize,
        sample_rate as usize,
    );
    let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .expect("Encode failed.");

    let mut sink = flacenc::bitsink::ByteSink::new();
    flac_stream.write(&mut sink);

    debug!("Finished compressing data into FLAC format");
    Ok(sink.as_slice().to_vec())
}

// Decompress FLAC data to WAV format using claxon crate
pub fn decompress_flac(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    debug!("Decompressing data from FLAC format...");

    let cursor = std::io::Cursor::new(buffer);
    let mut reader = FlacReader::new(cursor)?;

    let spec = WavSpec {
        channels: reader.streaminfo().channels as u16,
        sample_rate: reader.streaminfo().sample_rate,
        bits_per_sample: reader.streaminfo().bits_per_sample as u16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut samples = Vec::new();
    for sample in reader.samples() {
        let sample: i32 = sample?;
        samples.push(sample as i16);
    }

    debug!("Finished decompressing data from FLAC format");
    Ok((samples, spec))
}
