use db_rs::{View, payload_buffer::PayloadBuffer, views::hashmap::SHashMap};

#[test]
fn frames_preserve_boundaries_and_reject_truncation() {
    let mut buffer = PayloadBuffer::default();
    buffer.push(b"abc");
    buffer.push(b"");
    let bytes = std::mem::take(&mut buffer).bytes;
    assert!(buffer.bytes.is_empty());
    assert_eq!(&bytes[..8], &3u64.to_be_bytes());

    let (first, rest) = PayloadBuffer::head_payload(&bytes);
    assert_eq!(first, Some(b"abc".as_slice()));
    let (second, rest) = PayloadBuffer::head_payload(rest);
    assert_eq!(second, Some(b"".as_slice()));
    assert!(rest.is_empty());

    for end in 0..11 {
        let (payload, rest) = PayloadBuffer::head_payload(&bytes[..end]);
        assert!(payload.is_none());
        assert!(rest.is_empty());
    }
    assert!(
        PayloadBuffer::head_payload(&u64::MAX.to_be_bytes())
            .0
            .is_none()
    );
}

#[test]
fn pending_bytes_replay_without_queuing_new_events() {
    let mut original = SHashMap::<String, u32>::new();
    original.insert("old".into(), 1).unwrap();
    original.clear().unwrap();
    original.insert("keep".into(), 2).unwrap();
    original.insert("remove".into(), 3).unwrap();
    original.remove(&"remove".into()).unwrap();
    let bytes = original.take_events();
    assert!(original.take_events().is_empty());

    let mut recovered = SHashMap::<String, u32>::new();
    recovered.handle_events(&bytes).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered.get("keep"), Some(&2));
    assert!(recovered.take_events().is_empty());
    assert!(recovered.handle_events(&bytes[..bytes.len() - 1]).is_err());
}
