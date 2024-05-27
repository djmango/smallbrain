use ffmpeg_next as ffmpeg;
use hound::{WavReader, WavSpec, WavWriter};
use std::error::Error;
use std::io::{Cursor, Read, Write};
use std::process::{Command, Stdio};
use tracing::info;

pub fn compress_flac(
    samples: &[i16],
    spec: &WavSpec,
) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
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

pub fn decompress_flac(buffer: &[u8]) -> Result<(Vec<i16>, WavSpec), Box<dyn Error + Send + Sync>> {
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
