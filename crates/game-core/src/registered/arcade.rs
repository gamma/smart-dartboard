use super::RegisteredGameState;
use crate::GameError;
use sdb_contracts::{DartEvent, Ring};
use serde_json::{Value, json};

#[derive(Clone)]
pub(super) struct Target {
    pub label: String,
    pub field: u8,
    pub ring: Ring,
    pub multiplier: u8,
    pub score: u16,
}

pub(super) fn sample_targets(
    state: &mut RegisteredGameState,
    count: usize,
    difficulty: &str,
) -> Result<Vec<Target>, GameError> {
    let mut available = target_pool(difficulty);
    let mut targets = Vec::with_capacity(count);
    for _ in 0..count.min(available.len()) {
        let index = state.random_index(available.len())?;
        targets.push(available.remove(index));
    }
    Ok(targets)
}

pub(super) fn target_value(target: &Target) -> Value {
    json!({
        "label": target.label,
        "field": target.field,
        "ring": target.ring,
        "multiplier": target.multiplier,
        "score": target.score,
    })
}

pub(super) fn parse_target(value: &Value) -> Result<Target, GameError> {
    serde_json::from_value::<DartEvent>(json!({
        "type": "hit",
        "seq": 0,
        "field": value.get("field"),
        "ring": value.get("ring"),
        "multiplier": value.get("multiplier"),
        "label": value.get("label"),
        "score": value.get("score"),
    }))
    .map_err(|_| invalid_target())
    .and_then(|event| match event {
        DartEvent::Hit {
            label,
            field,
            ring,
            multiplier,
            score,
            ..
        } => Ok(Target {
            label,
            field,
            ring,
            multiplier,
            score,
        }),
        DartEvent::Miss { .. } => Err(invalid_target()),
    })
}

pub(super) fn same_field(event: &DartEvent, target: &Target) -> bool {
    matches!(event, DartEvent::Hit { field, .. } if *field == target.field)
}

pub(super) fn same_target(event: &DartEvent, target: &Target) -> bool {
    matches!(event, DartEvent::Hit { field, ring, .. } if *field == target.field && *ring == target.ring)
}

pub(super) fn zone_id(target: &Target) -> String {
    match target.ring {
        Ring::SingleBull => "SBULL".into(),
        Ring::DoubleBull => "DBULL".into(),
        _ => target.label.clone(),
    }
}

pub(super) const fn ring_name(ring: Ring) -> &'static str {
    match ring {
        Ring::SingleInner => "single_inner",
        Ring::SingleOuter => "single_outer",
        Ring::Triple => "triple",
        Ring::Double => "double",
        Ring::SingleBull => "single_bull",
        Ring::DoubleBull => "double_bull",
    }
}

pub(super) fn target_pool(difficulty: &str) -> Vec<Target> {
    let mut targets = Vec::new();
    if difficulty == "easy" || difficulty == "normal" {
        targets.extend((1..=20).map(|field| target_for(field, Ring::SingleOuter)));
    }
    if difficulty != "easy" {
        targets.extend((1..=20).map(|field| target_for(field, Ring::Double)));
        targets.extend((1..=20).map(|field| target_for(field, Ring::Triple)));
        targets.push(target_for(25, Ring::DoubleBull));
    }
    targets
}

pub(super) fn physical_target_pool() -> Vec<Target> {
    let mut targets = Vec::with_capacity(82);
    for field in 1..=20 {
        for ring in [
            Ring::SingleInner,
            Ring::Triple,
            Ring::SingleOuter,
            Ring::Double,
        ] {
            targets.push(target_for(field, ring));
        }
    }
    targets.push(target_for(25, Ring::SingleBull));
    targets.push(target_for(25, Ring::DoubleBull));
    targets
}

fn target_for(field: u8, ring: Ring) -> Target {
    let (prefix, multiplier) = match ring {
        Ring::Double => ("D", 2),
        Ring::Triple => ("T", 3),
        Ring::DoubleBull => ("DBull", 2),
        _ => ("S", 1),
    };
    Target {
        label: match (field, ring) {
            (25, Ring::SingleBull) => "SBull".into(),
            (25, Ring::DoubleBull) => "DBull".into(),
            _ => format!("{prefix}{field}"),
        },
        field,
        ring,
        multiplier,
        score: u16::from(field) * u16::from(multiplier),
    }
}

fn invalid_target() -> GameError {
    GameError::RulesetUnavailable("invalid arcade target".into())
}
