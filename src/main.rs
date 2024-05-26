use hound::WavReader;
use plotters::prelude::*;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fs::create_dir_all;

// mod decode;
// mod encode;

fn read_wav_file(file_path: &str) -> Result<Vec<f64>, hound::Error> {
    let mut reader = WavReader::open(file_path)?;
    let samples: Vec<f64> = reader.samples::<i16>().map(|s| s.unwrap() as f64).collect();

    // Normalize samples to range [-1.0, 1.0]
    let max_amplitude = i16::MAX as f64;
    let normalized_samples = samples.iter().map(|&s| s / max_amplitude).collect();

    Ok(normalized_samples)
}

fn plot_wav_data(samples: &[f64], output_file: &str) -> Result<(), Box<dyn Error>> {
    let root_area = BitMapBackend::new(output_file, (1024, 768)).into_drawing_area();
    root_area.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root_area)
        .caption("Audio Waveform", ("sans-serif", 50).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..samples.len(), -1.0..1.0)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        samples.iter().enumerate().map(|(x, y)| (x, *y)),
        &BLUE,
    ))?;

    root_area.present()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let data_dir = "data";
    let output_dir = "viz";

    // Create output directory if it doesn't exist
    create_dir_all(output_dir)?;

    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(OsStr::new("wav")) {
            let path_str = path.to_str().unwrap();
            match read_wav_file(path_str) {
                Ok(samples) => {
                    let file_stem = path.file_stem().unwrap().to_str().unwrap();
                    let output_file = format!("{}/{}.png", output_dir, file_stem);
                    if let Err(e) = plot_wav_data(&samples, &output_file) {
                        eprintln!("Error plotting WAV data for file {}: {}", path_str, e);
                    } else {
                        println!("WAV data plotted successfully for file {}", path_str);
                    }
                }
                Err(e) => {
                    eprintln!("Error reading WAV file {}: {}", path_str, e);
                }
            }
        }
    }
    Ok(())
}
