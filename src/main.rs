extern crate hound;
use hound::{WavReader, WavSpec, WavWriter};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process::Command;

mod riff;

fn read_wav_file(file_path: &str) -> Result<(Vec<i16>, WavSpec), Box<dyn Error>> {
    println!("Reading WAV file from {}", file_path);
    let mut reader = WavReader::open(file_path)?;
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    let spec = reader.spec();
    println!("Read {} samples", samples.len());
    Ok((samples, spec))
}

fn write_wav_file(output_path: &str, samples: &[i16], spec: WavSpec) -> Result<(), Box<dyn Error>> {
    println!("Writing WAV file to {}", output_path);
    let mut writer = WavWriter::create(output_path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    println!("Finished writing WAV file to {}", output_path);
    Ok(())
}

fn compress(samples: &[i16], output_path: &str, spec: &WavSpec) -> Result<(), Box<dyn Error>> {
    println!("Writing compressed data to {}", output_path);
    let mut file = BufWriter::new(File::create(output_path)?);

    // Write the metadata
    writeln!(file, "{}", samples.len())?;
    writeln!(file, "{}", spec.sample_rate)?;
    writeln!(file, "{}", spec.bits_per_sample)?;
    writeln!(file, "{}", spec.channels)?;

    // Write the samples
    for &sample in samples {
        writeln!(file, "{}", sample)?;
    }

    println!("Finished writing compressed data to {}", output_path);
    Ok(())
}

fn decompress(input_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    println!("Reading compressed data from {}", input_path);
    let file = BufReader::new(File::open(input_path)?);
    let mut lines = file.lines();

    // Read the metadata
    let total_samples: usize = lines.next().unwrap()?.trim().parse()?;
    let sample_rate: u32 = lines.next().unwrap()?.trim().parse()?;
    let bits_per_sample: u16 = lines.next().unwrap()?.trim().parse()?;
    let channels: u16 = lines.next().unwrap()?.trim().parse()?;

    println!(
        "Decompressing {} samples with spec: sample_rate = {}, bits_per_sample = {}, channels = {}",
        total_samples, sample_rate, bits_per_sample, channels
    );

    // Read the samples
    let samples: Vec<i16> = lines
        .take(total_samples)
        .map(|line| line.unwrap().trim().parse().unwrap())
        .collect();

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    write_wav_file(output_path, &samples, spec)?;
    println!("Finished decompressing to {}", output_path);
    Ok(())
}

fn process_batch(input_dir: &str) -> Result<(), Box<dyn Error>> {
    let start = std::time::Instant::now();
    println!("Removing existing data directory...");
    fs::remove_dir_all(input_dir).ok(); // This will ignore the error if the directory does not exist

    println!("Unzipping data.zip...");
    Command::new("unzip").arg("data.zip").output()?;

    let data_dir = Path::new(input_dir);

    let mut total_size_raw = 0;
    let mut total_size_compressed = 0;

    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().unwrap() == "wav" {
            let file_path = path.to_str().unwrap();
            println!("Processing {}", file_path);
            let compressed_file_path = format!("{}.brainwire", file_path);
            let decompressed_file_path = format!("{}.copy", file_path);

            let (samples, spec) = read_wav_file(file_path)?;
            compress(&samples, &compressed_file_path, &spec)?;
            decompress(&compressed_file_path, &decompressed_file_path)?;

            let file_size = fs::metadata(file_path)?.len();
            let compressed_size = fs::metadata(&compressed_file_path)?.len();

            if fs::read(file_path)? == fs::read(&decompressed_file_path)? {
                println!(
                    "{} losslessly compressed from {} bytes to {} bytes",
                    file_path, file_size, compressed_size
                );
            } else {
                eprintln!(
                    "ERROR: {} and {} are different.",
                    file_path, decompressed_file_path
                );
                return Err(Box::from(
                    "Decompressed file is different from the original.",
                ));
            }

            total_size_raw += file_size;
            total_size_compressed += compressed_size;
        }
    }

    let compression_ratio = total_size_raw as f64 / total_size_compressed as f64;

    println!("All recordings successfully compressed.");
    println!("Original size (bytes): {}", total_size_raw);
    println!("Compressed size (bytes): {}", total_size_compressed);
    println!("Compression ratio: {:.2}", compression_ratio);
    println!("Time taken: {:.2?}", start.elapsed());

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
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
            compress(&samples, output_path, &spec)?;
        }
        "decompress" => {
            if args.len() < 4 {
                eprintln!("Usage: {} decompress <input_file> <output_wav>", args[0]);
                std::process::exit(1);
            }
            let input_path = &args[2];
            let output_path = &args[3];
            decompress(input_path, output_path)?;
        }
        "process_batch" => {
            if args.len() < 3 {
                eprintln!("Usage: {} process_batch <input_dir>", args[0]);
                std::process::exit(1);
            }
            let input_dir = &args[2];
            process_batch(input_dir)?;
        }
        "riff" => {
            riff::main()?;
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }

    Ok(())
}
