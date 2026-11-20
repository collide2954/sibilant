mod fill;

use std::error::Error;
use std::fmt;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

use crate::fill::fill;

const SCRATCH_FRAMES: usize = 4_096;

pub struct OutputConfig {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
}

pub struct OutputStream {
    _stream: cpal::Stream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceError(String);

impl OutputConfig {
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate()
    }

    pub fn channels(&self) -> u16 {
        self.config.channels()
    }
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for DeviceError {}

pub fn default_output() -> Result<OutputConfig, DeviceError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| DeviceError("there is no audio output device".to_owned()))?;
    let config = device
        .default_output_config()
        .map_err(|error| DeviceError(error.to_string()))?;
    Ok(OutputConfig { device, config })
}

pub fn start_output(
    output: &OutputConfig,
    render: impl FnMut(&mut [f32]) + Send + 'static,
) -> Result<OutputStream, DeviceError> {
    let stream = match output.config.sample_format() {
        SampleFormat::F32 => build::<f32>(output, render),
        SampleFormat::I16 => build::<i16>(output, render),
        SampleFormat::I32 => build::<i32>(output, render),
        SampleFormat::U16 => build::<u16>(output, render),
        format => Err(DeviceError(format!(
            "the sample format {format:?} is not supported"
        ))),
    }?;
    stream
        .play()
        .map_err(|error| DeviceError(error.to_string()))?;
    Ok(OutputStream { _stream: stream })
}

fn build<T>(
    output: &OutputConfig,
    mut render: impl FnMut(&mut [f32]) + Send + 'static,
) -> Result<cpal::Stream, DeviceError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(output.config.channels());
    let mut scratch = vec![0.0; SCRATCH_FRAMES * channels];
    output
        .device
        .build_output_stream(
            output.config.config(),
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                fill(data, &mut scratch, &mut render);
            },
            |error| eprintln!("audio output error: {error}"),
            None,
        )
        .map_err(|error| DeviceError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    use crate::{default_output, start_output};

    #[test]
    #[ignore = "needs an audio output device"]
    fn the_default_output_asks_for_audio() {
        let output = default_output().unwrap();
        let rendered = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&rendered);
        let stream = start_output(&output, move |buffer| {
            buffer.fill(0.0);
            counter.fetch_add(buffer.len(), Ordering::Relaxed);
        })
        .unwrap();
        thread::sleep(Duration::from_millis(300));
        drop(stream);
        assert!(rendered.load(Ordering::Relaxed) > 0);
        assert!(output.sample_rate() > 0 && output.channels() > 0);
    }
}
