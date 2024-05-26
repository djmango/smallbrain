use brotli::CompressorWriter;
use brotli::Decompressor;
use ffmpeg_next as ffmpeg;
use hound::{WavReader, WavSpec, WavWriter};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn initialize_tracing(enable_logs: bool) {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(if enable_logs {
            Level::INFO
        } else {
            Level::ERROR
        })
        .without_time()
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

fn read_wav_file(file_path: &str) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    info!("Reading WAV file from {}", file_path);
    let mut reader = WavReader::open(file_path)?;
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    let spec = reader.spec();

    info!("Read {} samples", samples.len());
    Ok((samples, spec))
}

fn write_wav_file(
    output_path: &str,
    samples: &[i16],
    spec: WavSpec,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("Writing WAV file to {}", output_path);
    let mut writer = WavWriter::create(output_path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    info!("Finished writing WAV file to {}", output_path);
    Ok(())
}

fn compress_flac(samples: &[i16], spec: &WavSpec) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    info!("Compressing data into FLAC format...");

    // Initialize the FFmpeg library
    ffmpeg::init().expect("Could not initialize ffmpeg");

    // Prepare WAV data in memory using Cursor
    let mut wav_data = Cursor::new(Vec::new());
    {
        let mut writer = WavWriter::new(&mut wav_data, *spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
    }

    // Use ffmpeg to convert WAV to FLAC in memory
    let mut child = Command::new("ffmpeg")
        .args(&["-f", "wav", "-i", "pipe:0", "-f", "flac", "pipe:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        // Write WAV data to ffmpeg's stdin
        let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
        stdin.write_all(&wav_data.get_ref())?;
    }
    // Important: Close the child's stdin to signal that we are done writing
    drop(child.stdin.take());

    // Read FLAC data from ffmpeg's stdout
    let mut flac_data = Vec::new();
    {
        let stdout = child.stdout.as_mut().ok_or("Failed to open stdout")?;
        stdout.read_to_end(&mut flac_data)?;
    }

    // Capture stderr
    let mut stderr = String::new();
    if let Some(ref mut err) = child.stderr {
        err.read_to_string(&mut stderr).ok();
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("ffmpeg failed to convert WAV to FLAC. Error: {}", stderr).into());
    }

    info!("Finished compressing data into FLAC format");
    Ok(flac_data)
}

fn decompress_flac(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    info!("Decompressing data from FLAC format...");

    // Initialize the FFmpeg library
    ffmpeg::init().expect("Could not initialize ffmpeg");

    // Use ffmpeg to convert FLAC to WAV in memory
    let mut child = Command::new("ffmpeg")
        .args(&["-f", "flac", "-i", "pipe:0", "-f", "wav", "pipe:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        // Write FLAC data to ffmpeg's stdin
        let stdin = child.stdin.as_mut().ok_or("Failed to open stdin")?;
        stdin.write_all(buffer)?;
    }
    // Important: Close the child's stdin to signal that we are done writing
    drop(child.stdin.take());

    // Read WAV data from ffmpeg's stdout
    let mut wav_data = Vec::new();
    {
        let stdout = child.stdout.as_mut().ok_or("Failed to open stdout")?;
        stdout.read_to_end(&mut wav_data)?;
    }

    // Capture stderr
    let mut stderr = String::new();
    if let Some(ref mut err) = child.stderr {
        err.read_to_string(&mut stderr).ok();
    }

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("ffmpeg failed to convert FLAC to WAV. Error: {}", stderr).into());
    }

    // Parse WAV data
    let mut cursor = Cursor::new(&wav_data);
    let mut reader = WavReader::new(&mut cursor)?;
    let spec = reader.spec();
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();

    info!("Finished decompressing data from FLAC format");
    Ok((samples, spec))
}

fn compress_zlib(samples: &[i16], spec: &WavSpec) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    info!("Compressing data into zlib format...");

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

    info!("Finished compressing data into zlib format");
    Ok(compressed_data)
}

fn decompress_zlib(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    info!("Decompressing data from zlib format...");

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

    info!("Finished decompressing data from zlib format");
    Ok((samples, spec))
}

fn compress_brotli(
    samples: &[i16],
    spec: &WavSpec,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    info!("Compressing data into Brotli format...");

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

    info!("Finished compressing data into Brotli format");
    Ok(compressed_data)
}

fn decompress_brotli(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    info!("Decompressing data from Brotli format...");

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

    info!("Finished decompressing data from Brotli format");
    Ok((samples, spec))
}

fn compress(samples: &[i16], spec: &WavSpec) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    compress_brotli(samples, spec)
}

fn decompress(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    decompress_brotli(buffer)
}

fn process_batch(input_dir: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let start = std::time::Instant::now();
    info!("Removing existing data directory...");
    fs::remove_dir_all(input_dir).ok(); // This will ignore the error if the directory does not exist

    info!("Unzipping data.zip...");
    Command::new("unzip").arg("data.zip").output()?;

    let data_dir = Path::new(input_dir);

    let entries: Vec<_> = fs::read_dir(data_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("wav"))
        .collect();

    let bar = ProgressBar::new(entries.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar().template("{pos}/{len} [{elapsed}] - {wide_bar} {msg}")?,
    );

    let failed_files = Arc::new(Mutex::new(vec![]));

    // Process each entry in parallel
    let results: Vec<Result<(u64, u64), Box<dyn Error + Send + Sync>>> = entries
        .par_iter()
        .map(|entry| {
            let path = entry.path();
            let file_path = path.to_str().unwrap();
            let decompressed_file_path = format!("{}.copy", file_path);
            let failed_files = Arc::clone(&failed_files);

            let result = (|| {
                info!("Processing {}", file_path);

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

                let is_equal = fs::read(file_path)? == fs::read(&decompressed_file_path)?;
                if is_equal {
                    info!(
                        "{} losslessly compressed from {} bytes to {} bytes",
                        file_path, file_size, compressed_size
                    );
                    Ok((file_size, compressed_size))
                } else {
                    Err(Box::from(format!(
                        "ERROR: {} and {} are different.",
                        file_path, decompressed_file_path
                    )))
                }
            })();

            bar.inc(1);

            if let Err(ref e) = result {
                bar.println(format!("Error processing {}: {}", file_path, e));
                failed_files.lock().unwrap().push(file_path.to_string());
            }

            result
        })
        .collect();

    bar.finish_and_clear();

    // Aggregate results
    let (total_size_raw, total_size_compressed) = results
        .iter()
        .filter_map(|res| res.as_ref().ok())
        .fold((0u64, 0u64), |acc, &(fs, cs)| (acc.0 + fs, acc.1 + cs));

    if results.iter().any(Result::is_err) {
        for err in results.iter().filter_map(|res| res.as_ref().err()) {
            eprintln!("{}", err);
        }

        let failed_files = failed_files.lock().unwrap();
        if !failed_files.is_empty() {
            eprintln!("The following files failed to process:");
            for file in failed_files.iter() {
                eprintln!("{}", file);
            }
        }

        return Err(Box::from("Some files failed to be processed."));
    }

    let compression_ratio = total_size_raw as f64 / total_size_compressed as f64;

    info!("All recordings successfully compressed.");
    info!("Original size (bytes): {}", total_size_raw);
    info!("Compressed size (bytes): {}", total_size_compressed);
    info!("Compression ratio: {:.2}", compression_ratio);
    info!("Time taken: {:.2?}", start.elapsed());

    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage:\n  To compress:   {} compress <input_wav> <output_file>\n  To decompress: {} decompress <input_file> <output_wav>\n  To process batch: {} process_batch <input_dir> [--enable-logs]",
            args[0], args[0], args[0]
        );
        std::process::exit(1);
    }

    // Add a flag to enable logs
    let enable_logs = args.contains(&"--enable-logs".to_string());
    initialize_tracing(enable_logs);

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
                eprintln!(
                    "Usage: {} process_batch <input_dir> [--enable-logs]",
                    args[0]
                );
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
