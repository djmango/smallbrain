use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use hound::{WavReader, WavSpec, WavWriter};
use std::error::Error;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use tracing::debug;

pub fn compress_zlib(
    samples: &[i16],
    spec: &WavSpec,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    debug!("Compressing data into zlib format...");

    // Prepare WAV data in memory using Cursor
    let mut wav_data = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut wav_data, *spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }

    // Compress WAV data using zlib
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&wav_data.get_ref())?;
    let compressed_data = encoder.finish()?;

    debug!("Finished compressing data into zlib format");
    Ok(compressed_data)
}

pub fn decompress_zlib(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    debug!("Decompressing data from zlib format...");

    // Decompress zlib data to WAV in memory
    let mut decompressed_data = Vec::new();
    {
        let mut decoder = ZlibDecoder::new(buffer);
        decoder.read_to_end(&mut decompressed_data)?;
    }

    // Parse WAV data
    let mut cursor = Cursor::new(&decompressed_data);
    let mut reader = WavReader::new(&mut cursor)?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    debug!("Finished decompressing data from zlib format");
    Ok((samples, spec))
}
