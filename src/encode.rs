use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

fn run_length_encode(input: Vec<u8>) -> Vec<u8> {
    let mut encoded = vec![];
    let mut i = 0;

    while i < input.len() {
        let mut count = 1;

        while i + 1 < input.len() && input[i] == input[i + 1] {
            count += 1;
            i += 1;
        }

        encoded.push(count);
        encoded.push(input[i]);

        i += 1;
    }

    encoded
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: encode <input_wav> <output_file>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let mut input_file = File::open(input_path)?;
    let mut buffer = Vec::new();
    input_file.read_to_end(&mut buffer)?;

    let encoded = run_length_encode(buffer);

    let mut output_file = File::create(output_path)?;
    output_file.write_all(&encoded)?;

    Ok(())
}
