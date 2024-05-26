use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use hound::{WavReader, WavSpec, WavWriter};
use rayon::prelude::*;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;
use std::process::Command;

fn read_wav_file(file_path: &str) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    println!("Reading WAV file from {}", file_path);
    let mut reader = WavReader::open(file_path)?;
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    let spec = reader.spec();
    println!("Read {} samples", samples.len());
    Ok((samples, spec))
}

fn write_wav_file(
    output_path: &str,
    samples: &[i16],
    spec: WavSpec,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("Writing WAV file to {}", output_path);
    let mut writer = WavWriter::create(output_path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    println!("Finished writing WAV file to {}", output_path);
    Ok(())
}

fn compress(samples: &[i16], spec: &WavSpec) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    println!("Compressing data into memory...");
    let mut buffer = Vec::new();

    let mut encoder = ZlibEncoder::new(&mut buffer, Compression::default());

    // Write the metadata
    encoder.write_all(&(samples.len() as u32).to_le_bytes())?;
    encoder.write_all(&spec.sample_rate.to_le_bytes())?;
    encoder.write_all(&spec.bits_per_sample.to_le_bytes())?;
    encoder.write_all(&spec.channels.to_le_bytes())?;

    // Write the samples
    for &sample in samples {
        encoder.write_all(&sample.to_le_bytes())?;
    }

    encoder.finish()?;
    println!("Finished compressing data into memory");
    Ok(buffer)
}

fn decompress(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    println!("Decompressing data from memory...");
    let mut cursor = Cursor::new(buffer);
    let mut decoder = ZlibDecoder::new(&mut cursor);

    // Read the metadata
    let mut len_bytes = [0u8; 4];
    decoder.read_exact(&mut len_bytes)?;
    let total_samples = u32::from_le_bytes(len_bytes) as usize;

    let mut sample_rate_bytes = [0u8; 4];
    decoder.read_exact(&mut sample_rate_bytes)?;
    let sample_rate = u32::from_le_bytes(sample_rate_bytes);

    let mut bits_per_sample_bytes = [0u8; 2];
    decoder.read_exact(&mut bits_per_sample_bytes)?;
    let bits_per_sample = u16::from_le_bytes(bits_per_sample_bytes);

    let mut channels_bytes = [0u8; 2];
    decoder.read_exact(&mut channels_bytes)?;
    let channels = u16::from_le_bytes(channels_bytes);

    println!(
        "Decompressing {} samples with spec: sample_rate = {}, bits_per_sample = {}, channels = {}",
        total_samples, sample_rate, bits_per_sample, channels
    );

    // Read the samples
    let mut samples = Vec::with_capacity(total_samples);
    for _ in 0..total_samples {
        let mut sample_bytes = [0u8; 2];
        decoder.read_exact(&mut sample_bytes)?;
        samples.push(i16::from_le_bytes(sample_bytes));
    }

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    println!("Finished decompressing data from memory");
    Ok((samples, spec))
}

fn process_batch(input_dir: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = std::time::Instant::now();
    println!("Removing existing data directory...");
    fs::remove_dir_all(input_dir).ok(); // This will ignore the error if the directory does not exist

    println!("Unzipping data.zip...");
    Command::new("unzip").arg("data.zip").output()?;

    let data_dir = Path::new(input_dir);

    let entries: Vec<_> = fs::read_dir(data_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("wav"))
        .collect();

    // Process each entry in parallel
    let results: Vec<Result<(u64, u64), Box<dyn Error + Send + Sync>>> = entries
        .par_iter()
        .map(|entry| {
            let path = entry.path();
            let file_path = path.to_str().unwrap();
            println!("Processing {}", file_path);
            let decompressed_file_path = format!("{}.copy", file_path);

            let (samples, spec) = read_wav_file(file_path)?;
            let compressed_data = compress(&samples, &spec)?;
            let (decompressed_samples, decompressed_spec) = decompress(&compressed_data)?;

            write_wav_file(
                &decompressed_file_path,
                &decompressed_samples,
                decompressed_spec,
            )?;

            let file_size = fs::metadata(file_path)?.len();
            let compressed_size = compressed_data.len() as u64;

            if fs::read(file_path)? == fs::read(&decompressed_file_path)? {
                println!(
                    "{} losslessly compressed from {} bytes to {} bytes",
                    file_path, file_size, compressed_size
                );
                Ok((file_size, compressed_size))
            } else {
                eprintln!(
                    "ERROR: {} and {} are different.",
                    file_path, decompressed_file_path
                );
                Err(Box::from(
                    "Decompressed file is different from the original.",
                ))
            }
        })
        .collect();

    // Aggregate results
    let (total_size_raw, total_size_compressed) = results
        .iter()
        .filter_map(|res| res.as_ref().ok())
        .fold((0u64, 0u64), |acc, &(fs, cs)| (acc.0 + fs, acc.1 + cs));

    if results.iter().any(Result::is_err) {
        return Err(Box::from("Some files failed to be processed."));
    }

    let compression_ratio = total_size_raw as f64 / total_size_compressed as f64;

    println!("All recordings successfully compressed.");
    println!("Original size (bytes): {}", total_size_raw);
    println!("Compressed size (bytes): {}", total_size_compressed);
    println!("Compression ratio: {:.2}", compression_ratio);
    println!("Time taken: {:.2?}", start.elapsed());

    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage:\n  To compress:   {} compress <input_wav> <output_file>\n  To decompress: {} decompress <input_file> <output_wav>\n  To process batch: {} process_batch <input_dir>",
            args[0], args[0], args[0]
        );
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "compress" => {
            if args.len() < 4 {
                eprintln!("Usage: {} compress <input_wav> <output_file>", args[0]);
                std::process::exit(1);
            }
            let input_path = &args[2];
            let output_path = &args[3];
            let (samples, spec) = read_wav_file(input_path)?;
            let compressed_data = compress(&samples, &spec)?;
            let mut file = BufWriter::new(File::create(output_path)?);
            file.write_all(&compressed_data)?;
        }
        "decompress" => {
            if args.len() < 4 {
                eprintln!("Usage: {} decompress <input_file> <output_wav>", args[0]);
                std::process::exit(1);
            }
            let input_path = &args[2];
            let output_path = &args[3];
            let mut file = BufReader::new(File::open(input_path)?);
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            let (samples, spec) = decompress(&buffer)?;
            write_wav_file(output_path, &samples, spec)?;
        }
        "process_batch" => {
            if args.len() < 3 {
                eprintln!("Usage: {} process_batch <input_dir>", args[0]);
                std::process::exit(1);
            }
            let input_dir = &args[2];
            process_batch(input_dir)?;
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}
