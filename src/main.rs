extern crate hound;

use hound::{WavReader, WavSpec, WavWriter};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};

mod riff;

fn read_wav_file(file_path: &str) -> Result<(Vec<i16>, WavSpec), Box<dyn Error>> {
    println!("Reading WAV file from {}", file_path);
    let mut reader = WavReader::open(file_path)?;
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
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
    Ok(())
}

fn compress(samples: &[i16], output_path: &str) -> Result<(), Box<dyn Error>> {
    println!("Writing compressed data to {}", output_path);
    let mut file = File::create(output_path)?;
    write!(file, "{}", samples.len())?; // Write the total number of samples
    Ok(())
}

fn decompress(
    input_path: &str,
    output_path: &str,
    sample_rate: u32,
    bits_per_sample: u16,
    channels: u16,
) -> Result<(), Box<dyn Error>> {
    println!("Reading compressed data from {}", input_path);
    let mut file = File::open(input_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let _total_samples: usize = content.trim().parse()?; // Use _total_samples to avoid unused variable warning
    println!("Decompressing {} samples", _total_samples);

    // For testing, create placeholder data of the same length
    let samples: Vec<i16> = vec![0; _total_samples];
    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format: hound::SampleFormat::Int,
    };
    write_wav_file(output_path, &samples, spec)?;
    Ok(())
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
            let (samples, _) = read_wav_file(input_path)?;
            compress(&samples, output_path)?;
        }
        "decompress" => {
            // Note: Swap the order of arguments to match the function's signature
            let reader = WavReader::open(input_path)?;
            let spec = reader.spec();
            decompress(
                input_path,
                output_path,
                spec.sample_rate,
                spec.bits_per_sample,
                spec.channels,
            )?;
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
