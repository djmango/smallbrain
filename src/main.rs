extern crate hound;

use hound::WavReader;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Read, Write};

fn read_wav_file(file_path: &str) -> Result<Vec<f64>, Box<dyn Error>> {
    // Open the WAV file using hound
    println!("Reading WAV file from {}", file_path);
    let mut reader = WavReader::open(file_path)?;

    // Extract samples as f64 for processing
    let samples: Vec<f64> = reader.samples::<i16>().map(|s| s.unwrap() as f64).collect();

    Ok(samples)
}

fn write_wav_file(
    output_path: &str,
    samples: &[f64],
    sample_rate: u32,
    bits_per_sample: u16,
    channels: u16,
) -> Result<(), Box<dyn Error>> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(output_path, spec)?;
    for sample in samples {
        writer.write_sample(*sample as i16)?;
    }
    writer.finalize()?;
    Ok(())
}

fn point_slope_compress(samples: &[f64]) -> Vec<(usize, f64, f64)> {
    let mut compressed = vec![];
    let mut start = 0;

    while start < samples.len() {
        let mut end = start + 1;
        while end < samples.len() && samples[start] == samples[end] {
            end += 1;
        }

        if end >= samples.len() {
            break;
        }

        let slope = (samples[end] - samples[start]) / (end - start) as f64;
        compressed.push((start, samples[start], slope));
        start = end;
    }

    compressed
}

fn point_slope_decompress(compressed: &[(usize, f64, f64)], total_samples: usize) -> Vec<f64> {
    let mut samples = vec![0.0; total_samples];

    for segment in compressed {
        let (start, start_val, slope) = *segment;
        for i in 0..total_samples - start {
            samples[start + i] = start_val + slope * i as f64;
        }
    }

    samples
}

fn write_compressed_data(compressed: &[(usize, f64, f64)], output_path: &str) -> io::Result<()> {
    let mut file = File::create(output_path)?;
    for segment in compressed {
        writeln!(file, "{} {} {}", segment.0, segment.1, segment.2)?;
    }
    Ok(())
}

fn read_compressed_data(input_path: &str) -> io::Result<Vec<(usize, f64, f64)>> {
    let mut file = File::open(input_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut compressed = vec![];
    for line in content.lines() {
        let mut iter = line.split_whitespace();
        let start = iter.next().unwrap().parse().unwrap();
        let start_val = iter.next().unwrap().parse().unwrap();
        let slope = iter.next().unwrap().parse().unwrap();
        compressed.push((start, start_val, slope));
    }
    Ok(compressed)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 4 {
        eprintln!("Usage:\n  To compress:   {} compress <input_wav> <output_file>\n  To decompress: {} decompress <input_file> <output_wav>", args[0], args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    match command.as_str() {
        "compress" => {
            let samples = read_wav_file(input_path)?;
            let compressed = point_slope_compress(&samples);
            write_compressed_data(&compressed, output_path)?;
        }
        "decompress" => {
            let compressed = read_compressed_data(input_path)?;
            let reader = WavReader::open(input_path)?;
            let spec = reader.spec();
            let total_samples = reader.len() as usize;
            let samples = point_slope_decompress(&compressed, total_samples);
            write_wav_file(
                output_path,
                &samples,
                spec.sample_rate,
                spec.bits_per_sample,
                spec.channels,
            )?;
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
    Ok(())
}
