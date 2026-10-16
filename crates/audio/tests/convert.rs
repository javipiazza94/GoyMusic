use goymusic_audio::convert::Converter;

fn sine_f32(rate: u32, freq: f64, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| ((i as f64 * freq * std::f64::consts::TAU / f64::from(rate)).sin() * 0.5) as f32)
        .collect()
}

/// Upward zero crossings: a frequency estimate that ignores amplitude.
fn crossings(mono: &[f32]) -> usize {
    mono.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
}

fn convert_all(c: &mut Converter, input: &[f32], block: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for chunk in input.chunks(block) {
        out.extend(c.push(chunk).unwrap());
    }
    out.extend(c.finish().unwrap());
    out
}

#[test]
fn same_format_passes_through_untouched() {
    let input = sine_f32(44_100, 440.0, 5_000);
    let mut c = Converter::new(44_100, 1, 44_100, 1).unwrap();
    assert_eq!(convert_all(&mut c, &input, 777), input);
}

#[test]
fn mono_is_duplicated_and_stereo_is_averaged() {
    let mut up = Converter::new(48_000, 1, 48_000, 2).unwrap();
    assert_eq!(up.push(&[0.1, 0.2]).unwrap(), vec![0.1, 0.1, 0.2, 0.2]);
    let mut down = Converter::new(48_000, 2, 48_000, 1).unwrap();
    assert_eq!(down.push(&[0.2, 0.4, -1.0, 1.0]).unwrap(), vec![0.3_f32, 0.0]);
}

#[test]
fn resampling_keeps_duration_and_pitch() {
    for (from, to) in [(44_100u32, 48_000u32), (48_000, 44_100), (22_050, 48_000)] {
        let seconds = 2;
        let input = sine_f32(from, 1000.0, from as usize * seconds);
        let mut c = Converter::new(from, 1, to, 1).unwrap();
        let out = convert_all(&mut c, &input, 1_000);
        assert_eq!(out.len(), to as usize * seconds, "{from}->{to}: exact length");
        let measured = crossings(&out) as i64;
        assert!(
            (measured - 2_000).abs() <= 2,
            "{from}->{to}: 1 kHz for 2 s, got {measured} cycles"
        );
        // no delay at the start: the first output samples follow the input's sine, not silence
        let first_peak = out[..to as usize / 1000].iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!(first_peak > 0.4, "{from}->{to}: resampler delay not trimmed");
    }
}
