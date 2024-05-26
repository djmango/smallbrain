extern crate hound;

use hound::WavReader;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

fn get_wav_metadata(file_path: &Path) -> Result<String, Box<dyn Error>> {
    let reader = WavReader::open(file_path)?;
    let spec = reader.spec();
    let metadata = format!(
        "File: {}\nSample Rate: {}\nBits per Sample: {}\nChannels: {}\nLength: {}\n-----------\n",
        file_path.display(),
        spec.sample_rate,
        spec.bits_per_sample,
        spec.channels,
        reader.duration()
    );
    Ok(metadata)
}

fn read_metadata_file(metadata_file: &Path) -> io::Result<HashSet<String>> {
    let file = File::open(metadata_file)?;
    let reader = BufReader::new(file);

    let existing_metadata: HashSet<String> = reader.lines().map_while(|line| line.ok()).collect();

    Ok(existing_metadata)
}

fn write_metadata_file(metadata_file: &Path, new_metadata: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(metadata_file)?;

    writeln!(file, "{}", new_metadata)?;
    Ok(())
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <wav_directory> <metadata_output_file>", args[0]);
        std::process::exit(1);
    }

    let current_dir = env::current_dir()?;
    let wav_directory = current_dir.join(&args[2]); // Correct index for directory argument
    let metadata_file_path = current_dir.join(&args[3]); // Correct index for metadata file argument

    let existing_metadata = read_metadata_file(&metadata_file_path)?;

    for entry in fs::read_dir(wav_directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wav") {
            let metadata = get_wav_metadata(&path)?; // Pass the reference to Path

            if !existing_metadata.contains(&metadata) {
                write_metadata_file(&metadata_file_path, &metadata)?;
                println!("New metadata written for: {}", path.display());
            } else {
                println!("Metadata already exists for: {}", path.display());
            }
        }
    }

    Ok(())
}
