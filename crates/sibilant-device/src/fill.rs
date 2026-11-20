use cpal::{FromSample, Sample};

pub(crate) fn fill<T>(output: &mut [T], scratch: &mut [f32], render: &mut impl FnMut(&mut [f32]))
where
    T: Sample + FromSample<f32>,
{
    for chunk in output.chunks_mut(scratch.len()) {
        let rendered = &mut scratch[..chunk.len()];
        render(rendered);
        for (target, sample) in chunk.iter_mut().zip(rendered.iter()) {
            *target = T::from_sample(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fill::fill;

    #[test]
    fn renders_floats_straight_into_the_output() {
        let mut output = [0.0_f32; 6];
        let mut scratch = [0.0_f32; 4];
        let mut next = 0.0;
        fill(&mut output, &mut scratch, &mut |buffer: &mut [f32]| {
            for sample in buffer {
                next += 0.125;
                *sample = next;
            }
        });
        assert_eq!(output, [0.125, 0.25, 0.375, 0.5, 0.625, 0.75]);
    }

    #[test]
    fn converts_samples_to_the_format_of_the_device() {
        let mut output = [0_i16; 4];
        let mut scratch = [0.0_f32; 4];
        fill(&mut output, &mut scratch, &mut |buffer: &mut [f32]| {
            buffer.copy_from_slice(&[1.0, -1.0, 0.0, 0.5]);
        });
        assert_eq!(output, [i16::MAX, i16::MIN, 0, 16_384]);
    }

    #[test]
    fn renders_a_long_output_in_pieces_of_the_scratch_size() {
        let mut output = [0.0_f32; 10];
        let mut scratch = [0.0_f32; 4];
        let mut lengths = Vec::new();
        fill(&mut output, &mut scratch, &mut |buffer: &mut [f32]| {
            lengths.push(buffer.len());
        });
        assert_eq!(lengths, [4, 4, 2]);
    }
}
