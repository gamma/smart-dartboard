use sdb_protocol::{decode_packet, normalize_hex};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Fixture {
    fixture_schema_version: u16,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    hex: String,
    expected: Value,
}

#[test]
fn decoder_matches_shared_golden_fixture() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../fixtures/packets/fff1_decoder_v1.json"
    ))
    .expect("valid fixture");
    assert_eq!(fixture.fixture_schema_version, 1);

    for case in fixture.cases {
        let bytes = normalize_hex(&case.hex).expect("valid fixture hex");
        let actual = serde_json::to_value(decode_packet(&bytes)).expect("serialize packet");
        assert_eq!(actual, case.expected, "fixture case {}", case.name);
    }
}
