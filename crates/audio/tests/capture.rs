use goymusic_audio::sink::{AudioSink, CaptureSink, SinkError, SinkFormat};

const STEREO: SinkFormat = SinkFormat {
    sample_rate: 48_000,
    channels: 2,
};

#[test]
fn plays_instantly_and_applies_volume() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    assert_eq!(sink.write(&[0.5, 0.5]), 2);
    sink.set_volume(0.5);
    sink.write(&[0.5, 0.5]);
    assert_eq!(handle.played(), vec![0.5, 0.5, 0.25, 0.25]);
    assert_eq!(sink.played_frames(), 2);
}

#[test]
fn pause_holds_samples_and_clear_drops_them() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    sink.set_paused(true);
    sink.write(&[0.1, 0.1, 0.2, 0.2]);
    assert_eq!((sink.queued_frames(), sink.played_frames()), (2, 0));
    sink.clear();
    sink.write(&[0.3, 0.3]);
    sink.set_paused(false);
    assert_eq!(handle.played(), vec![0.3, 0.3]);
    assert_eq!(handle.clears(), 1);
}

#[test]
fn injected_failure_is_reported_once() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    handle.fail(SinkError::Device("unplugged".into()));
    assert_eq!(sink.take_error(), Some(SinkError::Device("unplugged".into())));
    assert_eq!(sink.take_error(), None);
}

#[test]
fn hold_mode_plays_only_on_demand() {
    let (mut sink, handle) = CaptureSink::new(STEREO);
    handle.hold(true);
    sink.write(&[0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
    assert_eq!((sink.queued_frames(), sink.played_frames()), (3, 0));
    handle.play(2, 2);
    assert_eq!(handle.played(), vec![0.1, 0.1, 0.2, 0.2]);
    assert_eq!(sink.queued_frames(), 1);
}
