//! Rust side of the Rust × TS widget contract (spec 0003 §2.6): every JSON
//! in `tests/fixtures/widgets/` reads into the Rust types, passes the
//! rules, and writes back to exactly the same JSON. The TS side checks the
//! same files against its own types, so changing one side alone breaks one
//! of the two tests.
//!
//! A unit test rather than one in `tests/`: the domain is private to the
//! crate.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::calendar::CardPeriod;
use super::clock::{ClockIconStyle, HourCycle};
use super::cycle::Anchor;
use super::shortcut::ShortcutTarget;
use super::system::{ActivityShow, ActivityStyle};
use super::time_progress::{Period, WeekStart};
use super::*;

/// Fixtures that are not a widget instance, each with its own type.
/// `ipc.json` is checked in `commands/widgets.rs`, next to the commands.
const ENUMS: &str = "enums.json";
const IPC: &str = "ipc.json";
const STORAGE_STATUSES: &str = "storage-statuses.json";
const SNAPSHOT: &str = "snapshot-sample.json";

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/widgets")
}

/// Every `.json` in the fixtures folder, by file name, sorted.
fn fixture_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixtures_dir())
        .expect("tests/fixtures/widgets is readable")
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".json"))
        .collect();
    names.sort();
    names
}

fn read_fixture(name: &str) -> Value {
    let text = fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// Reads `name` as a `T` and checks that writing it back gives the same
/// JSON: a field renamed, added or dropped on either side shows up here.
fn round_trip<T: Serialize + DeserializeOwned>(name: &str) -> T {
    let expected = read_fixture(name);
    let value: T = serde_json::from_value(expected.clone())
        .unwrap_or_else(|error| panic!("{name} doesn't read as the Rust type: {error}"));
    let written = serde_json::to_value(&value).unwrap();
    assert_eq!(written, expected, "{name} doesn't write back the same");
    value
}

fn assert_valid(instance: &WidgetInstance, name: &str) {
    assert_eq!(
        validate_label(instance.label.as_deref()),
        Ok(()),
        "{name}: label"
    );
    assert_eq!(instance.config.validate(), Ok(()), "{name}: config");
}

#[test]
fn every_instance_fixture_round_trips_and_is_valid() {
    let mut kinds = BTreeSet::new();
    let instances: Vec<String> = fixture_names()
        .into_iter()
        .filter(|name| ![ENUMS, IPC, STORAGE_STATUSES, SNAPSHOT].contains(&name.as_str()))
        .collect();
    assert!(!instances.is_empty(), "no instance fixture found");

    for name in &instances {
        let instance: WidgetInstance = round_trip(name);
        assert_valid(&instance, name);
        kinds.insert(instance.kind().as_str());

        let stem = name.strip_suffix(".json").unwrap();
        assert!(
            stem.starts_with(instance.kind().as_str()),
            "{name} holds a {} instance",
            instance.kind()
        );
        if stem.ends_with("-default") {
            assert_eq!(
                instance.config,
                WidgetConfig::default_for(instance.kind()),
                "{name} is not the kind's default"
            );
        }
    }

    let all: BTreeSet<_> = WidgetKind::ALL.iter().map(|kind| kind.as_str()).collect();
    assert_eq!(kinds, all, "every kind has at least one fixture");
}

#[test]
fn the_snapshot_fixture_round_trips_and_is_valid() {
    let snapshot: WidgetsSnapshot = round_trip(SNAPSHOT);
    assert!(!snapshot.widgets.is_empty());
    for instance in &snapshot.widgets {
        assert_valid(instance, SNAPSHOT);
    }
}

/// Every storage status: each state, and `unavailable` with each reason.
fn every_storage_status() -> Vec<StorageStatus> {
    // Never called: a new status doesn't compile until it is listed below.
    const _: fn(StorageStatus) = |status| match status {
        StorageStatus::Ok | StorageStatus::Recovered | StorageStatus::Unavailable { .. } => {}
    };
    let mut all = vec![StorageStatus::Ok, StorageStatus::Recovered];
    all.extend(
        UnavailableReason::ALL
            .into_iter()
            .map(|reason| StorageStatus::Unavailable { reason }),
    );
    all
}

/// One target of each type.
fn every_shortcut_target() -> Vec<ShortcutTarget> {
    // Never called: a new type doesn't compile until it is listed below.
    const _: fn(ShortcutTarget) = |target| match target {
        ShortcutTarget::Path { .. } | ShortcutTarget::Url { .. } => {}
    };
    vec![
        ShortcutTarget::default(),
        ShortcutTarget::Url {
            url: "https://example.com/".to_owned(),
        },
    ]
}

/// The value of the `field` tag of each of `values`, in order, each once.
fn tags<T: Serialize>(values: &[T], field: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for value in values {
        let tag = serde_json::to_value(value).unwrap()[field]
            .as_str()
            .unwrap_or_else(|| panic!("no `{field}` tag"))
            .to_owned();
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// Reads the list `name` of `enums.json` as `T`s: it must be exactly `all`.
fn assert_closed_set<T>(enums: &Value, name: &str, all: &[T])
where
    T: DeserializeOwned + PartialEq + Debug,
{
    let listed: Vec<T> = serde_json::from_value(enums[name].clone())
        .unwrap_or_else(|error| panic!("{ENUMS}: {name} doesn't read as the Rust enum: {error}"));
    assert_eq!(listed, all, "{ENUMS}: {name}");
}

#[test]
fn the_storage_status_fixture_round_trips_and_lists_every_status() {
    let statuses: Vec<StorageStatus> = round_trip(STORAGE_STATUSES);
    assert_eq!(statuses, every_storage_status());
}

#[test]
fn the_enum_fixture_is_every_value_of_each_closed_set_in_order() {
    let enums = read_fixture(ENUMS);
    let expected = json!({
        "WidgetKind": WidgetKind::ALL,
        "HourCycle": HourCycle::ALL,
        "ClockIconStyle": ClockIconStyle::ALL,
        "Period": Period::ALL,
        "WeekStart": WeekStart::ALL,
        "MoveDirection": MoveDirection::ALL,
        "UnavailableReason": UnavailableReason::ALL,
        "StorageState": tags(&every_storage_status(), "state"),
        "ShortcutTargetType": tags(&every_shortcut_target(), "type"),
        "ActivityShow": ActivityShow::ALL,
        "ActivityStyle": ActivityStyle::ALL,
        "CardPeriod": CardPeriod::ALL,
        "ReminderAnchor": Anchor::ALL,
        "WidgetErrorCode": WidgetErrorCode::ALL,
    });
    assert_eq!(enums, expected, "{ENUMS} doesn't match the Rust enums");

    // And every name in it reads back as the same value.
    assert_closed_set(&enums, "WidgetKind", &WidgetKind::ALL);
    assert_closed_set(&enums, "HourCycle", &HourCycle::ALL);
    assert_closed_set(&enums, "ClockIconStyle", &ClockIconStyle::ALL);
    assert_closed_set(&enums, "Period", &Period::ALL);
    assert_closed_set(&enums, "WeekStart", &WeekStart::ALL);
    assert_closed_set(&enums, "MoveDirection", &MoveDirection::ALL);
    assert_closed_set(&enums, "UnavailableReason", &UnavailableReason::ALL);
    assert_closed_set(&enums, "ActivityShow", &ActivityShow::ALL);
    assert_closed_set(&enums, "ActivityStyle", &ActivityStyle::ALL);
    assert_closed_set(&enums, "CardPeriod", &CardPeriod::ALL);
    assert_closed_set(&enums, "ReminderAnchor", &Anchor::ALL);
    assert_closed_set(&enums, "WidgetErrorCode", &WidgetErrorCode::ALL);
}
