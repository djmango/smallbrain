use brotli::CompressorWriter;
use brotli::Decompressor;
use hound::{WavReader, WavSpec, WavWriter};
use std::error::Error;
use std::io::{Cursor, Read, Write};
use tracing::{debug, info};

pub fn compress_brotli(
    samples: &[i16],
    spec: &WavSpec,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    debug!("Compressing data into Brotli format...");

    // Prepare WAV data in memory using Cursor
    let mut wav_data = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut wav_data, *spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }

    // Compress WAV data using Brotli
    let mut compressed_data = Vec::new();
    {
        let mut compressor = CompressorWriter::new(&mut compressed_data, 4096, 11, 22);
        compressor.write_all(&wav_data.get_ref())?;
    }

    debug!("Finished compressing data into Brotli format");
    Ok(compressed_data)
}

pub fn decompress_brotli(
    buffer: &[u8],
) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    debug!("Decompressing data from Brotli format...");

    // Decompress Brotli data to WAV in memory
    let mut decompressed_data = Vec::new();
    {
        let mut decompressor = Decompressor::new(buffer, 4096);
        decompressor.read_to_end(&mut decompressed_data)?;
    }

    // Parse WAV data
    let mut cursor = Cursor::new(&decompressed_data);
    let mut reader = WavReader::new(&mut cursor)?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    debug!("Finished decompressing data from Brotli format");
    Ok((samples, spec))
}
