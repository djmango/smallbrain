// use crate::brotli_sb::{compress_brotli, decompress_brotli};
// use crate::flac::{compress_flac, decompress_flac};
use crate::wav::{read_wav_file, write_wav_file};
use crate::zstd::{compress_zstd, decompress_zstd};
use hound::WavSpec;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::env;
use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;

mod brotli_sb;
mod flac;
mod tenbit;
mod wav;
mod zlib;
mod zstd;

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

fn compress(samples: &[i16], spec: &WavSpec) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    // let flac_data = compress_flac(samples, spec)?;
    // compress_brotli(&flac_data)
    compress_zstd(samples, spec)
}

fn decompress(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    // let flac_data = decompress_brotli(buffer)?;
    // decompress_flac(&flac_data)
    decompress_zstd(buffer)
}

fn print_diff(original: &[u8], decompressed: &[u8]) {
    let min_len = std::cmp::min(original.len(), decompressed.len());
    let mut diff_count = 0;
    let mut diff_output = String::new();

    for i in 0..min_len {
        if original[i] != decompressed[i] {
            writeln!(
                &mut diff_output,
                "Byte {}: original = {:02X}, decompressed = {:02X}",
                i, original[i], decompressed[i]
            )
            .unwrap();
            diff_count += 1;
            if diff_count > 20 {
                writeln!(
                    &mut diff_output,
                    "-- More differences follow, limit of 20 differences shown --"
                )
                .unwrap();
                break;
            }
        }
    }

    if original.len() > min_len {
        writeln!(
            &mut diff_output,
            "Original file has extra bytes starting from byte {}",
            min_len
        )
        .unwrap();
    } else if decompressed.len() > min_len {
        writeln!(
            &mut diff_output,
            "Decompressed file has extra bytes starting from byte {}",
            min_len
        )
        .unwrap();
    }

    println!("{}", diff_output);
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
                debug!("Processing {}", file_path);

                let (samples, spec) = read_wav_file(file_path)?;
                let compressed_data = compress(&samples, &spec)?;
                let (decompressed_samples, decompressed_spec) = decompress(&compressed_data)?;

                write_wav_file(
                    &decompressed_file_path,
                    &decompressed_samples,
                    decompressed_spec,
                )?;

                let original_contents = fs::read(file_path)?;
                let decompressed_contents = fs::read(&decompressed_file_path)?;

                let file_size = fs::metadata(file_path)?.len();
                let compressed_size = compressed_data.len() as u64;

                if original_contents == decompressed_contents {
                    debug!(
                        "{} losslessly compressed from {} bytes to {} bytes",
                        file_path, file_size, compressed_size
                    );
                    Ok((file_size, compressed_size))
                } else {
                    print_diff(&original_contents, &decompressed_contents);
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
