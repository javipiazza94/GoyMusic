use goymusic_core::events::{Event, EventBus};

#[test]
fn every_subscriber_receives_published_events() {
    let bus = EventBus::new();
    let a = bus.subscribe();
    let b = bus.subscribe();
    bus.publish(Event::ScanProgress { done: 1, total: 2 });
    assert_eq!(a.try_recv().unwrap(), Event::ScanProgress { done: 1, total: 2 });
    assert_eq!(b.try_recv().unwrap(), Event::ScanProgress { done: 1, total: 2 });
}

#[test]
fn dropped_subscribers_are_removed_on_publish() {
    let bus = EventBus::new();
    let kept = bus.subscribe();
    drop(bus.subscribe());
    bus.publish(Event::LibraryUpdated);
    assert_eq!(bus.subscriber_count(), 1);
    assert_eq!(kept.try_recv().unwrap(), Event::LibraryUpdated);
}

#[test]
fn events_serialize_with_a_type_tag() {
    let json = serde_json::to_string(&Event::ScanProgress { done: 3, total: 9 }).unwrap();
    assert_eq!(json, r#"{"type":"scanProgress","done":3,"total":9}"#);
}

#[test]
fn player_events_use_camel_case_fields() {
    use goymusic_core::events::PlaybackState;
    use goymusic_core::TrackId;
    let json = serde_json::to_string(&Event::PositionChanged {
        track: TrackId(4),
        position_ms: 1500,
    })
    .unwrap();
    assert_eq!(json, r#"{"type":"positionChanged","track":4,"positionMs":1500}"#);
    let json = serde_json::to_string(&Event::PlayerStateChanged {
        state: PlaybackState::Playing,
        track: None,
    })
    .unwrap();
    assert_eq!(json, r#"{"type":"playerStateChanged","state":"playing","track":null}"#);
}
