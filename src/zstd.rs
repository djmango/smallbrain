use hound::{WavReader, WavSpec, WavWriter};
use std::error::Error;
use std::io::{Cursor, Read, Write};
use tracing::debug;

pub fn compress_zstd(
    samples: &[i16],
    spec: &WavSpec,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    debug!("Compressing data into zstd format...");

    // Prepare WAV data in memory using Cursor
    let mut wav_data = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut wav_data, *spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }

    // Compress WAV data using zstd
    let mut encoder = zstd::Encoder::new(Vec::new(), 0)?; // 0 is the default compression level
    encoder.write_all(&wav_data.get_ref())?;
    let compressed_data = encoder.finish()?;

    debug!("Finished compressing data into zstd format");
    Ok(compressed_data)
}

pub fn decompress_zstd(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    debug!("Decompressing data from zstd format...");

    // Decompress zstd data to WAV in memory
    let mut decompressed_data = Vec::new();
    {
        let mut decoder = zstd::Decoder::new(buffer)?;
        decoder.read_to_end(&mut decompressed_data)?;
    }

    // Parse WAV data
    let mut cursor = Cursor::new(&decompressed_data);
    let mut reader = WavReader::new(&mut cursor)?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    debug!("Finished decompressing data from zstd format");
    Ok((samples, spec))
}
