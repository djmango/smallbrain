use hound::{WavReader, WavSpec, WavWriter};
use std::error::Error;
use tracing::{debug, info};

pub fn read_wav_file(file_path: &str) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
    debug!("Reading WAV file from {}", file_path);
    let mut reader = WavReader::open(file_path)?;
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    let spec = reader.spec();

    debug!("Read {} samples", samples.len());
    Ok((samples, spec))
}

pub fn write_wav_file(
    output_path: &str,
    samples: &[i16],
    spec: WavSpec,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    debug!("Writing WAV file to {}", output_path);
    let mut writer = WavWriter::create(output_path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    debug!("Finished writing WAV file to {}", output_path);
    Ok(())
}
