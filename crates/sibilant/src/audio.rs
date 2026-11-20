use std::fmt;

use sibilant_core::project::Project;
use sibilant_core::time::{Clock, Position};
use sibilant_device::{DeviceError, OutputStream};
use sibilant_engine::arrangement::Arrangement;
use sibilant_engine::engine::{self, Controller};
use sibilant_engine::timing::{self, Timing};

pub(crate) struct Audio {
    pub(crate) controller: Controller,
    sample_rate: u32,
    _stream: Option<OutputStream>,
}

impl Audio {
    pub(crate) fn start(project: &Project) -> Result<Self, DeviceError> {
        let output = sibilant_device::default_output()?;
        let (controller, mut engine) = engine::new(timing(project, output.sample_rate()));
        let channels = usize::from(output.channels());
        let stream =
            sibilant_device::start_output(&output, move |buffer| engine.process(buffer, channels))?;
        Ok(Self {
            controller,
            sample_rate: output.sample_rate(),
            _stream: Some(stream),
        })
    }

    pub(crate) fn playhead(&self) -> Position<Clock> {
        timing::clock_of_frame(self.sample_rate, self.controller.playhead())
    }

    #[cfg(test)]
    pub(crate) fn without_device(
        project: &Project,
        sample_rate: u32,
    ) -> (Self, sibilant_engine::engine::Engine) {
        let (controller, engine) = engine::new(timing(project, sample_rate));
        let audio = Self {
            controller,
            sample_rate,
            _stream: None,
        };
        (audio, engine)
    }
}

impl fmt::Debug for Audio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Audio")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

fn timing(project: &Project, sample_rate: u32) -> Arrangement {
    Arrangement::new(
        Timing::new(
            sample_rate,
            project.tempo_map().clone(),
            project.time_signature_map().clone(),
        ),
        Vec::new(),
    )
}
