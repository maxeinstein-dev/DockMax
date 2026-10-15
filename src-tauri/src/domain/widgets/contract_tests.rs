//! Rust side of the Rust × TS widget contract (spec 0003 §2.6): every JSON
//! in `tests/fixtures/widgets/` reads into the Rust types, passes the
//! rules, and writes back to exactly the same JSON. The TS side checks the
//! same files against its own types, so changing one side alone breaks one
//! of the two tests.
//!
//! A unit test rather than one in `tests/`: the domain is private to the
//! crate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::*;

/// Fixtures that are not a widget instance, each with its own type.
const ERROR_CODES: &str = "error-codes.json";
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
        .filter(|name| ![ERROR_CODES, STORAGE_STATUSES, SNAPSHOT].contains(&name.as_str()))
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

#[test]
fn the_storage_status_fixture_round_trips_and_lists_every_status() {
    let statuses: Vec<StorageStatus> = round_trip(STORAGE_STATUSES);
    let expected = [
        StorageStatus::Ok,
        StorageStatus::Recovered,
        StorageStatus::Unavailable {
            reason: UnavailableReason::NewerVersion,
        },
        StorageStatus::Unavailable {
            reason: UnavailableReason::MigrationFailed,
        },
        StorageStatus::Unavailable {
            reason: UnavailableReason::OpenFailed,
        },
    ];
    assert_eq!(statuses, expected);
}

#[test]
fn the_error_code_fixture_is_every_code_in_order() {
    let codes: Vec<WidgetErrorCode> = round_trip(ERROR_CODES);
    assert_eq!(codes, WidgetErrorCode::ALL);
}
