use brotli::CompressorWriter;
use brotli::Decompressor;
use std::error::Error;
use std::io::{Read, Write};

/// Compress data using Brotli
pub fn compress_brotli(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut compressed = Vec::new();
    {
        let mut compressor = CompressorWriter::new(&mut compressed, 4096, 11, 22);
        compressor.write_all(data)?;
        compressor.flush()?;
    }
    Ok(compressed)
}

/// Decompress data using Brotli
pub fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    let mut decompressed = Vec::new();
    let mut decompressor = Decompressor::new(data, 4096);
    decompressor.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

// pub fn compress_brotli(
//     samples: &[i16],
//     spec: &WavSpec,
// ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
//     debug!("Compressing data into Brotli format...");

//     // Scale samples to 10-bit values and then pack them
//     // let scaled_samples = scale_to_10_bits(samples);
//     // let packed_samples = pack_10_bit_values(&scaled_samples);

//     // let samples = packed_samples;

//     // Prepare WAV data in memory using Cursor
//     let mut wav_data = Cursor::new(Vec::new());
//     {
//         let mut writer = WavWriter::new(&mut wav_data, *spec)?;
//         for &sample in samples {
//             writer.write_sample(sample)?;
//         }
//         writer.finalize()?;
//     }

//     // Compress WAV data using Brotli
//     let mut compressed_data = Vec::new();
//     {
//         let mut compressor = CompressorWriter::new(&mut compressed_data, 4096, 11, 22);
//         compressor.write_all(&wav_data.get_ref())?;
//     }

//     debug!("Finished compressing data into Brotli format");
//     Ok(compressed_data)
// }

// // pub fn decompress_brotli(
// //     buffer: &[u8],
// // ) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
// //     debug!("Decompressing data from Brotli format...");

// //     // Decompress Brotli data to WAV in memory
// //     let mut decompressed_data = Vec::new();
// //     {
// //         let mut decompressor = Decompressor::new(buffer, 4096);
// //         decompressor.read_to_end(&mut decompressed_data)?;
// //     }

// //     // Parse WAV data
// //     let mut cursor = Cursor::new(&decompressed_data);
// //     let mut reader = WavReader::new(&mut cursor)?;
// //     let spec = reader.spec();
// //     let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

// //     debug!("Finished decompressing data from Brotli format");
// //     Ok((samples, spec))
// // }

// pub fn decompress_brotli(
//     compressed_data: &[u8],
// ) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
//     debug!("Decompressing data from Brotli format...");

//     let mut decompressed_data = Vec::new();
//     let mut decompressor = Decompressor::new(compressed_data, 4096);
//     decompressor.read_to_end(&mut decompressed_data)?;

//     // let packed_samples: &[u16] = bytemuck::cast_slice(&decompressed_data);
//     // let unpacked_samples = unpack_10_bit_values(packed_samples);

//     // // Scale back to 16-bit
//     // let scaled_back_samples: Vec<i16> = unpacked_samples
//     //     .iter()
//     //     .map(|&sample| (((sample as i32) << 6) + i16::MIN as i32) as i16)
//     //     .collect();

//     // let samples = scaled_back_samples;

//     // Parse WAV data
//     let mut cursor = Cursor::new(&decompressed_data);
//     let mut reader = WavReader::new(&mut cursor)?;
//     let spec = reader.spec();
//     let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

//     // let spec = WavSpec {
//     //     channels: 1,
//     //     sample_rate: 19531,
//     //     bits_per_sample: 16,
//     //     sample_format: hound::SampleFormat::Int,
//     // };

//     debug!("Finished decompressing data from Brotli format");
//     Ok((samples, spec))
// }
