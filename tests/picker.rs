use fz::{Candidates, Outcome, Picker, PickerConfig};

fn picker(multi: bool) -> Picker {
    Picker::new(Candidates::from_records(vec![b"alpha".to_vec(), b"alpha".to_vec(), b"beta".to_vec()]), PickerConfig {
        lines: 10, prompt: b"> ".to_vec(), query: vec![], show_scores: false,
        show_info: false, multi, workers: 1,
    })
}

fn type_bytes(picker: &mut Picker, bytes: &[u8]) -> Option<Outcome> {
    let mut result = None;
    for &byte in bytes { result = picker.feed(Some(byte)); }
    result
}

#[test]
fn stable_duplicate_ids_and_marking_order_survive_queries() {
    let mut picker = picker(true);
    assert!(type_bytes(&mut picker, b"\x1b[B\x1b[B \x1b[A ").is_none());
    type_bytes(&mut picker, b"zzz");
    let Outcome::Accepted(ids) = type_bytes(&mut picker, b"\r").unwrap() else { panic!("expected IDs"); };
    assert_eq!(ids.iter().map(|id| id.index()).collect::<Vec<_>>(), [1, 0]);
    assert_eq!(picker.source(ids[0]), Some(b"alpha".as_slice()));
}

#[test]
fn unmatched_query_cancel_and_no_match_are_distinct() {
    let mut single = picker(false);
    type_bytes(&mut single, b"missing");
    assert_eq!(single.feed(Some(b'\r')), Some(Outcome::Query(b"missing".to_vec())));
    let mut multi = picker(true);
    type_bytes(&mut multi, b"missing");
    assert_eq!(multi.feed(Some(b'\r')), Some(Outcome::NoMatch));
    assert_eq!(multi.feed(Some(3)), Some(Outcome::Cancelled));
}

#[test]
fn paste_cannot_accept_or_cancel() {
    let mut picker = picker(true);
    assert!(type_bytes(&mut picker, b"\x1b[200~alpha\r\x03\x1b[201~").is_none());
    let Outcome::Accepted(ids) = picker.feed(Some(b'\r')).unwrap() else { panic!("expected row"); };
    assert_eq!(ids[0].index(), 0);
}

#[test]
fn resize_refuses_invisible_multi_selection() {
    let mut picker = picker(true);
    assert!(picker.resize(80, 2).is_err());
    assert!(picker.resize(4, 24).is_err());
    assert!(picker.resize(5, 3).is_ok());
}
