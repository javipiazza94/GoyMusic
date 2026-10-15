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
