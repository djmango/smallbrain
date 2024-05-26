use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

fn run_length_decode(input: Vec<u8>) -> Vec<u8> {
    let mut decoded = vec![];
    let mut i = 0;

    while i < input.len() {
        let count = input[i];
        let value = input[i + 1];
        for _ in 0..count {
            decoded.push(value);
        }
        i += 2;
    }

    decoded
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: decode <input_file> <output_wav>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let mut input_file = File::open(input_path)?;
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)?;

    let decoded = run_length_decode(buffer);

    let mut output_file = File::create(output_path)?;
    output_file.write_all(&decoded)?;

    Ok(())
}
