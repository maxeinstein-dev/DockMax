//! The Clima (spec 0017 F5): the forecast of each city a dock's Clima
//! shows, asked of Open-Meteo every 30 minutes while any dock has one, and
//! again when its panel opens if the last answer is older than 10 minutes.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex, Once};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::domain::weather::{self, Forecast, Place};

const EVERY: Duration = Duration::from_secs(30 * 60);
/// An open panel asks again only past this.
const FRESH: Duration = Duration::from_secs(10 * 60);

/// What the widget shows for one city.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Reading {
    /// The forecast, and when it was read (Unix seconds).
    Ready { forecast: Forecast, at: i64 },
    /// Open-Meteo didn't answer, or not as expected.
    Unavailable,
}

/// A city: its coordinates in ten-thousandths of a degree.
pub type Key = (i32, i32);

pub fn key(latitude: f64, longitude: f64) -> Key {
    (
        (latitude * 10_000.0).round() as i32,
        (longitude * 10_000.0).round() as i32,
    )
}

/// Each city's last reading and when it was asked.
static STATE: Mutex<Option<HashMap<Key, (Reading, Instant)>>> = Mutex::new(None);
/// The cities each dock shows.
static WANTERS: Mutex<Vec<(isize, Vec<Key>)>> = Mutex::new(Vec::new());
static STARTED: Once = Once::new();
static WAKE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The last reading at the city `key`; `None` before the first.
pub fn reading(key: Key) -> Option<Reading> {
    lock(&STATE)
        .as_ref()?
        .get(&key)
        .map(|(reading, _)| reading.clone())
}

/// The cities the dock `dock` shows: their weather is read while any dock
/// shows one.
pub fn want(dock: isize, cities: Vec<Key>) {
    let (wanted, new) = {
        let mut wanters = lock(&WANTERS);
        wanters.retain(|(each, _)| *each != dock);
        let new = {
            let state = lock(&STATE);
            cities
                .iter()
                .any(|city| state.as_ref().is_none_or(|state| !state.contains_key(city)))
        };
        if !cities.is_empty() {
            wanters.push((dock, cities));
        }
        (!wanters.is_empty(), new)
    };
    if wanted {
        STARTED.call_once(|| {
            let _ = std::thread::Builder::new()
                .name("weather".into())
                .spawn(run);
        });
        if new {
            read_now();
        }
    }
}

/// Reads again the cities whose answer is older than 10 minutes (a panel
/// opened, or a city was picked).
pub fn read_now() {
    let (lock, wake) = &WAKE;
    *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
    wake.notify_all();
}

/// The cities named like `query`, for the settings' search.
pub fn search(query: &str) -> Result<Vec<Place>, ()> {
    let url = weather::search_url(query).ok_or(())?;
    match super::http::get(&url, None) {
        Ok(response) if response.status == 200 => weather::parse_places(&response.body).ok_or(()),
        Ok(response) => {
            log::warn!("weather: the search answered {}", response.status);
            Err(())
        }
        Err(error) => {
            log::warn!("weather: no answer to the search: {error:?}");
            Err(())
        }
    }
}

/// Asks Open-Meteo for the forecast at `city` once.
fn read(city: Key) -> Reading {
    let url = weather::forecast_url(f64::from(city.0) / 10_000.0, f64::from(city.1) / 10_000.0);
    match super::http::get(&url, None) {
        Ok(response) if response.status == 200 => {
            weather::parse_forecast(&response.body).map_or(Reading::Unavailable, |forecast| {
                Reading::Ready {
                    forecast,
                    at: jiff::Timestamp::now().as_second(),
                }
            })
        }
        Ok(response) => {
            log::warn!("weather: Open-Meteo answered {}", response.status);
            Reading::Unavailable
        }
        Err(error) => {
            log::warn!("weather: no answer: {error:?}");
            Reading::Unavailable
        }
    }
}

fn run() {
    let mut last: Option<Instant> = None;
    loop {
        {
            let (lock, wake) = &WAKE;
            let mut asked = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let wait = last.map_or(Duration::ZERO, |at| EVERY.saturating_sub(at.elapsed()));
            if !*asked && !wait.is_zero() {
                asked = wake
                    .wait_timeout(asked, wait)
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .0;
            }
            *asked = false;
        }
        let mut cities: Vec<Key> = lock(&WANTERS)
            .iter()
            .flat_map(|(_, cities)| cities.iter().copied())
            .collect();
        cities.sort_unstable();
        cities.dedup();
        // Only the cities never read, or not lately.
        cities.retain(|city| {
            lock(&STATE)
                .as_ref()
                .and_then(|state| state.get(city))
                .is_none_or(|(_, at)| at.elapsed() >= FRESH)
        });
        let mut changed = false;
        for city in cities {
            let next = read(city);
            let mut state = lock(&STATE);
            let state = state.get_or_insert_with(HashMap::new);
            changed |= state.get(&city).is_none_or(|(reading, _)| *reading != next);
            state.insert(city, (next, Instant::now()));
        }
        last = Some(Instant::now());
        if changed {
            super::native_dock::calendar_changed();
        }
    }
}
