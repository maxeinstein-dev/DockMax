//! The launcher's conversions (spec 0012 §5b): a query such as
//! `10 km em milhas`, `30c em f` or `100 usd em brl` shows the converted
//! value first, like the calculator. Its own parser, which only converts:
//! nothing typed is ever run. Pure.
//!
//! - `<number> <unit> (em|para|pra|to|in) <unit>`: one result, when both
//!   units measure the same thing (a length into a mass gives nothing, and
//!   the query goes on to the other providers).
//! - `<number> <unit>`: the most common conversions of that unit, at most
//!   [`MAX_COMMON`].
//! - Numbers take a comma or a point as the decimal mark, as the calculator.
//!   Units are named in Portuguese or English, without case or accents.
//! - Currencies by ISO code, a few names (`dólar`, `euro`, `real`, `libra`)
//!   and the symbols `R$`, `US$`, `$`, `€`, `£`, with the European Central
//!   Bank's rates as Frankfurter publishes them (fetched once a day by
//!   `windows::launcher::rates`): without rates, a currency gives nothing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::launcher::normalize;
use super::launcher_calc;

/// The most conversions a unit alone shows.
pub const MAX_COMMON: usize = 3;

/// Frankfurter's latest rates of the European Central Bank, against the
/// euro. (`api.frankfurter.app/latest` answers 301 to this address.)
pub const RATES_URL: &str = "https://api.frankfurter.dev/v1/latest";

/// The file the rates are kept in, in the app's data folder.
pub const CACHE_FILE: &str = "currency-rates.json";

/// The words between the two units.
const CONNECTORS: [&str; 8] = ["em", "para", "pra", "to", "in", "=", "->", "→"];

/// The currencies a currency alone is converted into, in this order.
const COMMON_CURRENCIES: [&str; 4] = ["BRL", "USD", "EUR", "GBP"];

/// What a unit measures; only units of the same one convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    Length,
    Mass,
    Temperature,
    Volume,
    Area,
    Speed,
    Data,
    Time,
}

/// A unit: its id (the key of its name in `i18n`), the symbol it is shown
/// with, how many of the quantity's base unit it is (unused for the
/// temperatures), the names it is typed with (normalized) and the units it
/// is converted into when typed alone.
#[derive(Debug, PartialEq)]
pub struct Unit {
    pub id: &'static str,
    pub symbol: &'static str,
    pub quantity: Quantity,
    factor: f64,
    names: &'static [&'static str],
    common: &'static [&'static str],
}

macro_rules! unit {
    ($id:literal, $symbol:literal, $quantity:ident, $factor:expr, [$($name:literal),*], [$($common:literal),*]) => {
        Unit {
            id: $id,
            symbol: $symbol,
            quantity: Quantity::$quantity,
            factor: $factor,
            names: &[$($name),*],
            common: &[$($common),*],
        }
    };
}

/// Every unit. The base units: the metre, the kilogram, the kelvin, the
/// litre, the square metre, the metre per second, the byte and the second.
static UNITS: &[Unit] = &[
    // Length.
    unit!(
        "mm",
        "mm",
        Length,
        0.001,
        [
            "mm",
            "milimetro",
            "milimetros",
            "millimeter",
            "millimeters",
            "millimetre",
            "millimetres"
        ],
        ["cm", "in"]
    ),
    unit!(
        "cm",
        "cm",
        Length,
        0.01,
        [
            "cm",
            "centimetro",
            "centimetros",
            "centimeter",
            "centimeters",
            "centimetre",
            "centimetres"
        ],
        ["in", "mm", "m"]
    ),
    unit!(
        "m",
        "m",
        Length,
        1.0,
        ["m", "metro", "metros", "meter", "meters", "metre", "metres"],
        ["ft", "cm", "yd"]
    ),
    unit!(
        "km",
        "km",
        Length,
        1000.0,
        [
            "km",
            "quilometro",
            "quilometros",
            "kilometro",
            "kilometros",
            "kilometer",
            "kilometers",
            "kilometre",
            "kilometres"
        ],
        ["mi", "m", "nmi"]
    ),
    unit!(
        "in",
        "in",
        Length,
        0.0254,
        ["in", "pol", "polegada", "polegadas", "inch", "inches"],
        ["cm", "mm", "ft"]
    ),
    unit!(
        "ft",
        "ft",
        Length,
        0.3048,
        ["ft", "pe", "pes", "foot", "feet"],
        ["m", "cm", "in"]
    ),
    unit!(
        "yd",
        "yd",
        Length,
        0.9144,
        ["yd", "jarda", "jardas", "yard", "yards"],
        ["m", "ft"]
    ),
    unit!(
        "mi",
        "mi",
        Length,
        1609.344,
        ["mi", "milha", "milhas", "mile", "miles"],
        ["km", "m"]
    ),
    unit!(
        "nmi",
        "nmi",
        Length,
        1852.0,
        [
            "nmi",
            "milha nautica",
            "milhas nauticas",
            "nautical mile",
            "nautical miles"
        ],
        ["km", "mi"]
    ),
    // Mass.
    unit!(
        "mg",
        "mg",
        Mass,
        1e-6,
        ["mg", "miligrama", "miligramas", "milligram", "milligrams"],
        ["g"]
    ),
    unit!(
        "g",
        "g",
        Mass,
        0.001,
        ["g", "grama", "gramas", "gram", "grams"],
        ["oz", "kg", "mg"]
    ),
    unit!(
        "kg",
        "kg",
        Mass,
        1.0,
        [
            "kg",
            "quilo",
            "quilos",
            "quilograma",
            "quilogramas",
            "kilo",
            "kilos",
            "kilogram",
            "kilograms"
        ],
        ["lb", "g"]
    ),
    unit!(
        "t",
        "t",
        Mass,
        1000.0,
        [
            "t",
            "tonelada",
            "toneladas",
            "ton",
            "tons",
            "tonne",
            "tonnes"
        ],
        ["kg", "lb"]
    ),
    unit!(
        "oz",
        "oz",
        Mass,
        0.028_349_523_125,
        ["oz", "onca", "oncas", "ounce", "ounces"],
        ["g"]
    ),
    unit!(
        "lb",
        "lb",
        Mass,
        0.453_592_37,
        ["lb", "lbs", "libra", "libras", "pound", "pounds"],
        ["kg", "g", "oz"]
    ),
    // Temperature: converted through the kelvin.
    unit!(
        "c",
        "°C",
        Temperature,
        1.0,
        [
            "c",
            "celsius",
            "grau",
            "graus",
            "grau celsius",
            "graus celsius",
            "degree celsius",
            "degrees celsius"
        ],
        ["f", "k"]
    ),
    unit!(
        "f",
        "°F",
        Temperature,
        1.0,
        [
            "f",
            "fahrenheit",
            "grau fahrenheit",
            "graus fahrenheit",
            "degree fahrenheit",
            "degrees fahrenheit"
        ],
        ["c", "k"]
    ),
    unit!(
        "k",
        "K",
        Temperature,
        1.0,
        ["k", "kelvin", "kelvins"],
        ["c", "f"]
    ),
    // Volume.
    unit!(
        "ml",
        "ml",
        Volume,
        0.001,
        [
            "ml",
            "mililitro",
            "mililitros",
            "milliliter",
            "milliliters",
            "millilitre",
            "millilitres"
        ],
        ["floz", "l", "cup"]
    ),
    unit!(
        "l",
        "l",
        Volume,
        1.0,
        [
            "l", "lt", "litro", "litros", "liter", "liters", "litre", "litres"
        ],
        ["gal", "ml", "cup"]
    ),
    unit!(
        "m3",
        "m³",
        Volume,
        1000.0,
        [
            "m3",
            "metro cubico",
            "metros cubicos",
            "cubic meter",
            "cubic meters",
            "cubic metre",
            "cubic metres"
        ],
        ["l", "gal"]
    ),
    unit!(
        "gal",
        "gal",
        Volume,
        3.785_411_784,
        [
            "gal", "gal us", "us gal", "galao", "galoes", "gallon", "gallons"
        ],
        ["l", "ml"]
    ),
    unit!(
        "floz",
        "fl oz",
        Volume,
        0.029_573_529_562_5,
        [
            "fl oz",
            "floz",
            "onca fluida",
            "oncas fluidas",
            "fluid ounce",
            "fluid ounces"
        ],
        ["ml"]
    ),
    // The US legal cup, 240 ml.
    unit!(
        "cup",
        "xícara",
        Volume,
        0.24,
        ["xicara", "xicaras", "cup", "cups"],
        ["ml", "floz"]
    ),
    // Area.
    unit!(
        "m2",
        "m²",
        Area,
        1.0,
        [
            "m2",
            "metro quadrado",
            "metros quadrados",
            "square meter",
            "square meters",
            "square metre",
            "square metres",
            "sq m"
        ],
        ["ft2", "ha"]
    ),
    unit!(
        "km2",
        "km²",
        Area,
        1e6,
        [
            "km2",
            "quilometro quadrado",
            "quilometros quadrados",
            "square kilometer",
            "square kilometers",
            "square kilometre",
            "square kilometres",
            "sq km"
        ],
        ["ha", "m2", "acre"]
    ),
    unit!(
        "ha",
        "ha",
        Area,
        10_000.0,
        ["ha", "hectare", "hectares"],
        ["acre", "m2", "km2"]
    ),
    unit!(
        "acre",
        "acre",
        Area,
        4_046.856_422_4,
        ["acre", "acres", "ac"],
        ["ha", "m2"]
    ),
    unit!(
        "ft2",
        "ft²",
        Area,
        0.092_903_04,
        [
            "ft2",
            "pe quadrado",
            "pes quadrados",
            "square foot",
            "square feet",
            "sq ft"
        ],
        ["m2"]
    ),
    // Speed.
    unit!(
        "kmh",
        "km/h",
        Speed,
        1.0 / 3.6,
        [
            "km/h",
            "kmh",
            "kph",
            "quilometro por hora",
            "quilometros por hora",
            "kilometer per hour",
            "kilometers per hour"
        ],
        ["mph", "ms", "kn"]
    ),
    unit!(
        "ms",
        "m/s",
        Speed,
        1.0,
        [
            "m/s",
            "metro por segundo",
            "metros por segundo",
            "meter per second",
            "meters per second"
        ],
        ["kmh", "mph"]
    ),
    unit!(
        "mph",
        "mph",
        Speed,
        0.447_04,
        [
            "mph",
            "milha por hora",
            "milhas por hora",
            "mile per hour",
            "miles per hour"
        ],
        ["kmh", "ms"]
    ),
    unit!(
        "kn",
        "kn",
        Speed,
        1852.0 / 3600.0,
        ["kn", "kt", "no", "nos", "knot", "knots"],
        ["kmh", "mph"]
    ),
    // Data: KB, MB… are powers of 1000; KiB, MiB… of 1024.
    unit!("b", "B", Data, 1.0, ["b", "byte", "bytes"], ["kb", "kib"]),
    unit!(
        "kb",
        "KB",
        Data,
        1e3,
        ["kb", "kilobyte", "kilobytes", "quilobyte", "quilobytes"],
        ["b", "kib", "mb"]
    ),
    unit!(
        "mb",
        "MB",
        Data,
        1e6,
        ["mb", "megabyte", "megabytes"],
        ["kb", "mib", "gb"]
    ),
    unit!(
        "gb",
        "GB",
        Data,
        1e9,
        ["gb", "gigabyte", "gigabytes"],
        ["mb", "gib", "tb"]
    ),
    unit!(
        "tb",
        "TB",
        Data,
        1e12,
        ["tb", "terabyte", "terabytes"],
        ["gb", "tib"]
    ),
    unit!(
        "kib",
        "KiB",
        Data,
        1024.0,
        ["kib", "kibibyte", "kibibytes"],
        ["kb", "b"]
    ),
    unit!(
        "mib",
        "MiB",
        Data,
        1_048_576.0,
        ["mib", "mebibyte", "mebibytes"],
        ["mb", "kib"]
    ),
    unit!(
        "gib",
        "GiB",
        Data,
        1_073_741_824.0,
        ["gib", "gibibyte", "gibibytes"],
        ["gb", "mib"]
    ),
    unit!(
        "tib",
        "TiB",
        Data,
        1_099_511_627_776.0,
        ["tib", "tebibyte", "tebibytes"],
        ["tb", "gib"]
    ),
    // Time.
    unit!(
        "s",
        "s",
        Time,
        1.0,
        [
            "s", "seg", "sec", "secs", "segundo", "segundos", "second", "seconds"
        ],
        ["min", "h"]
    ),
    unit!(
        "min",
        "min",
        Time,
        60.0,
        ["min", "mins", "minuto", "minutos", "minute", "minutes"],
        ["s", "h"]
    ),
    unit!(
        "h",
        "h",
        Time,
        3600.0,
        ["h", "hr", "hrs", "hora", "horas", "hour", "hours"],
        ["min", "s", "day"]
    ),
    unit!(
        "day",
        "d",
        Time,
        86_400.0,
        ["d", "dia", "dias", "day", "days"],
        ["h", "week", "min"]
    ),
    unit!(
        "week",
        "sem",
        Time,
        604_800.0,
        ["sem", "semana", "semanas", "wk", "week", "weeks"],
        ["day", "h"]
    ),
];

/// The names and symbols of a few currencies (normalized); any ISO code the
/// rates have also counts.
const CURRENCY_NAMES: &[(&str, &str)] = &[
    ("r$", "BRL"),
    ("us$", "USD"),
    ("$", "USD"),
    ("€", "EUR"),
    ("£", "GBP"),
    ("real", "BRL"),
    ("reais", "BRL"),
    ("dolar", "USD"),
    ("dolares", "USD"),
    ("dollar", "USD"),
    ("dollars", "USD"),
    ("euro", "EUR"),
    ("euros", "EUR"),
    ("libra", "GBP"),
    ("libras", "GBP"),
    ("libra esterlina", "GBP"),
    ("libras esterlinas", "GBP"),
    ("sterling", "GBP"),
    ("iene", "JPY"),
    ("ienes", "JPY"),
    ("yen", "JPY"),
];

/// The symbols a query may start with (`R$ 100 em usd`), the longest first.
const CURRENCY_PREFIXES: [&str; 5] = ["us$", "r$", "€", "£", "$"];

/// The European Central Bank's rates: how many of each currency one euro
/// buys (the euro itself included, at 1), and the day they are of
/// (`YYYY-MM-DD`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rates {
    pub date: String,
    pub rates: BTreeMap<String, f64>,
}

/// The rates as kept on disk: with the day (UTC, `YYYY-MM-DD`) they were
/// fetched, so they are fetched again only the next day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cache {
    pub fetched: String,
    pub rates: Rates,
}

/// What a value is measured in.
#[derive(Debug, Clone, PartialEq)]
pub enum Measure {
    Unit(&'static Unit),
    /// An ISO code, in capitals.
    Currency(String),
}

/// One conversion: `amount` of `from` is `value` of `to`; the rates' day
/// for a currency.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub amount: f64,
    pub from: Measure,
    pub to: Measure,
    pub value: f64,
    pub rates_date: Option<String>,
}

/// A conversion as the launcher shows it: "= 6,21 milhas" over "10 km", and
/// the number Enter copies ("6,21").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub title: String,
    pub subtitle: String,
    pub copied: String,
}

/// The conversions `query` asks for: none when it is not one, when the units
/// measure different things, or for a currency without `rates`.
pub fn convert(query: &str, rates: Option<&Rates>) -> Vec<Conversion> {
    let text = fold_symbols(&normalize(query));
    let (prefix, text) = match CURRENCY_PREFIXES
        .iter()
        .find(|symbol| text.starts_with(**symbol))
    {
        Some(symbol) => (
            currency_of_name(symbol).map(|code| Measure::Currency(code.to_owned())),
            text[symbol.len()..].trim_start(),
        ),
        None => (None, text.as_str()),
    };
    let Some((amount, rest)) = number(text) else {
        return Vec::new();
    };
    let glued = !rest.starts_with(' ');
    let rest = rest.trim();
    let words: Vec<&str> = rest.split(' ').filter(|word| !word.is_empty()).collect();

    // `<number> <unit> em <unit>`, at the first connector that splits it
    // into two units that convert.
    for at in 0..words.len() {
        if !CONNECTORS.contains(&words[at]) || at + 1 == words.len() {
            continue;
        }
        let froms = match (&prefix, at) {
            (Some(prefix), 0) => vec![prefix.clone()],
            (None, at) if at > 0 => lookup(&words[..at].join(" "), rates),
            _ => continue,
        };
        let tos = lookup(&words[at + 1..].join(" "), rates);
        for from in &froms {
            for to in &tos {
                if let Some(conversion) = between(amount, from, to, rates) {
                    return vec![conversion];
                }
            }
        }
    }

    // `<number> <unit>`: its most common conversions.
    let from = match (&prefix, words.is_empty()) {
        (Some(prefix), true) => prefix.clone(),
        (None, false) => {
            // `3d` is more likely an app's name than three days: a unit of
            // one letter glued to the number needs the `em`.
            if glued && rest.chars().count() == 1 {
                return Vec::new();
            }
            let Some(from) = lookup(rest, rates).into_iter().next() else {
                return Vec::new();
            };
            from
        }
        _ => return Vec::new(),
    };
    let targets: Vec<Measure> = match &from {
        Measure::Unit(unit) => unit
            .common
            .iter()
            .filter_map(|id| unit_by_id(id))
            .map(Measure::Unit)
            .collect(),
        Measure::Currency(code) => COMMON_CURRENCIES
            .iter()
            .filter(|each| *each != code)
            .map(|each| Measure::Currency((*each).to_owned()))
            .collect(),
    };
    targets
        .iter()
        .filter_map(|to| between(amount, &from, to, rates))
        .take(MAX_COMMON)
        .collect()
}

/// `conversion` as the launcher shows it, with `decimal` as the decimal
/// mark. `name` gives a unit's name by its id, plural or not ("milhas");
/// without one, its symbol. `rates_of` says whose rates a currency's are,
/// with `{d}`, `{m}` and `{y}` for the day.
pub fn show(
    conversion: &Conversion,
    decimal: char,
    name: impl Fn(&str, bool) -> Option<&'static str>,
    rates_of: &str,
) -> Shown {
    let amount = launcher_calc::format(conversion.amount, decimal);
    match (&conversion.from, &conversion.to) {
        (Measure::Currency(from), Measure::Currency(to)) => {
            let copied = money(conversion.value, decimal);
            let mut subtitle = format!("{amount} {from}");
            if let Some((y, m, d)) = conversion.rates_date.as_deref().and_then(day_parts) {
                subtitle.push_str(" · ");
                subtitle.push_str(
                    &rates_of
                        .replace("{d}", d)
                        .replace("{m}", m)
                        .replace("{y}", y),
                );
            }
            Shown {
                title: format!("= {copied} {to}"),
                subtitle,
                copied,
            }
        }
        (from, to) => {
            let value = round(conversion.value);
            let copied = launcher_calc::format(value, decimal);
            let (from, to) = (symbol(from), unit_name(to, value, &name));
            Shown {
                title: format!("= {copied} {to}"),
                subtitle: format!("{amount} {from}"),
                copied,
            }
        }
    }
}

/// The rates of Frankfurter's answer (`{"amount":1.0,"base":"EUR",
/// "date":"2026-10-30","rates":{"USD":1.1172,…}}`), the euro added at 1.
/// `None` when it is not one, or not against the euro.
pub fn parse_rates(body: &[u8]) -> Option<Rates> {
    #[derive(Deserialize)]
    struct Latest {
        base: String,
        date: String,
        rates: BTreeMap<String, f64>,
    }
    let latest: Latest = serde_json::from_slice(body).ok()?;
    if latest.base != "EUR" || day_parts(&latest.date).is_none() || latest.rates.is_empty() {
        return None;
    }
    let mut rates = latest.rates;
    if !rates
        .iter()
        .all(|(code, rate)| is_code(code) && rate.is_finite() && *rate > 0.0)
    {
        return None;
    }
    rates.insert("EUR".to_owned(), 1.0);
    Some(Rates {
        date: latest.date,
        rates,
    })
}

/// The rates kept on disk, when the file is one.
pub fn read_cache(text: &str) -> Option<Cache> {
    let cache: Cache = serde_json::from_str(text).ok()?;
    let valid = day_parts(&cache.fetched).is_some()
        && day_parts(&cache.rates.date).is_some()
        && cache
            .rates
            .rates
            .iter()
            .all(|(code, rate)| is_code(code) && rate.is_finite() && *rate > 0.0);
    valid.then_some(cache)
}

/// The rates as kept on disk.
pub fn write_cache(cache: &Cache) -> String {
    serde_json::to_string(cache).unwrap_or_default()
}

/// Whether the rates kept were fetched `today` (UTC, `YYYY-MM-DD`): then
/// they are not fetched again.
pub fn is_fresh(cache: Option<&Cache>, today: &str) -> bool {
    cache.is_some_and(|cache| cache.fetched == today)
}

/// The rates kept after a fetch on `today`: the new ones; without them (no
/// network, a bad answer), the old ones, even from another day; without
/// either, none, and the currencies give nothing.
pub fn after_fetch(old: Option<Cache>, fetched: Option<Rates>, today: &str) -> Option<Cache> {
    match fetched {
        Some(rates) => Some(Cache {
            fetched: today.to_owned(),
            rates,
        }),
        None => old,
    }
}

/// Whether one of `conversions` is of a currency.
pub fn is_currency(conversions: &[Conversion]) -> bool {
    conversions
        .iter()
        .any(|conversion| matches!(conversion.from, Measure::Currency(_)))
}

/// `°`, `º` dropped and `²`, `³` as digits: `30°C` is `30c`, `m²` is `m2`.
fn fold_symbols(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '°' | 'º'))
        .map(|c| match c {
            '²' => '2',
            '³' => '3',
            other => other,
        })
        .collect()
}

/// The number `text` starts with (a sign, digits and one decimal mark) and
/// what comes after it.
fn number(text: &str) -> Option<(f64, &str)> {
    let mut end = 0;
    let mut digits = String::new();
    let mut mark = false;
    for (at, c) in text.char_indices() {
        match c {
            '-' | '+' if at == 0 => digits.push(c),
            '0'..='9' => digits.push(c),
            ',' | '.' if !mark => {
                mark = true;
                digits.push('.');
            }
            _ => break,
        }
        end = at + c.len_utf8();
    }
    if !digits.bytes().any(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let value: f64 = digits.parse().ok()?;
    value.is_finite().then_some((value, &text[end..]))
}

/// What `name` (normalized) can be: the units first, then the currencies.
fn lookup(name: &str, rates: Option<&Rates>) -> Vec<Measure> {
    let mut found: Vec<Measure> = UNITS
        .iter()
        .filter(|unit| unit.names.contains(&name))
        .map(Measure::Unit)
        .collect();
    if let Some(code) = currency_of_name(name) {
        found.push(Measure::Currency(code.to_owned()));
    } else if name.len() == 3 && name.bytes().all(|byte| byte.is_ascii_lowercase()) {
        let code = name.to_ascii_uppercase();
        if rates.is_some_and(|rates| rates.rates.contains_key(&code)) {
            found.push(Measure::Currency(code));
        }
    }
    found
}

fn currency_of_name(name: &str) -> Option<&'static str> {
    CURRENCY_NAMES
        .iter()
        .find(|(each, _)| *each == name)
        .map(|(_, code)| *code)
}

fn unit_by_id(id: &str) -> Option<&'static Unit> {
    UNITS.iter().find(|unit| unit.id == id)
}

/// `amount` of `from` in `to`, when they convert.
fn between(amount: f64, from: &Measure, to: &Measure, rates: Option<&Rates>) -> Option<Conversion> {
    let value = match (from, to) {
        (Measure::Unit(from), Measure::Unit(to)) if from.quantity == to.quantity => {
            from_base(to, to_base(from, amount))
        }
        (Measure::Currency(from), Measure::Currency(to)) => {
            let rates = &rates?.rates;
            amount / rates.get(from)? * rates.get(to)?
        }
        _ => return None,
    };
    value.is_finite().then(|| Conversion {
        amount,
        from: from.clone(),
        to: to.clone(),
        value,
        rates_date: match from {
            Measure::Currency(_) => rates.map(|rates| rates.date.clone()),
            Measure::Unit(_) => None,
        },
    })
}

/// `value` of `unit` in the quantity's base unit (the kelvin for the
/// temperatures, which are not proportional).
fn to_base(unit: &Unit, value: f64) -> f64 {
    match unit.id {
        "c" => value + 273.15,
        "f" => (value - 32.0) * 5.0 / 9.0 + 273.15,
        _ => value * unit.factor,
    }
}

fn from_base(unit: &Unit, value: f64) -> f64 {
    match unit.id {
        "c" => value - 273.15,
        "f" => (value - 273.15) * 9.0 / 5.0 + 32.0,
        _ => value / unit.factor,
    }
}

/// A unit's value as shown: two decimals from 1 up; below it, three
/// significant digits (at most ten decimals), so `1 mm em km` is not 0.
fn round(value: f64) -> f64 {
    let size = value.abs();
    let decimals = if size >= 1.0 || size == 0.0 {
        2
    } else {
        (2 - size.log10().floor() as i32).clamp(2, 10)
    };
    let scale = 10f64.powi(decimals);
    let rounded = (value * scale).round() / scale;
    if rounded.is_finite() { rounded } else { value }
}

/// A sum of money: always two decimals (`532,10`).
fn money(value: f64, decimal: char) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    format!("{rounded:.2}").replace('.', &decimal.to_string())
}

fn symbol(measure: &Measure) -> String {
    match measure {
        Measure::Unit(unit) => unit.symbol.to_owned(),
        Measure::Currency(code) => code.clone(),
    }
}

fn unit_name(
    measure: &Measure,
    value: f64,
    name: &impl Fn(&str, bool) -> Option<&'static str>,
) -> String {
    match measure {
        Measure::Unit(unit) => name(unit.id, value.abs() != 1.0)
            .unwrap_or(unit.symbol)
            .to_owned(),
        Measure::Currency(code) => code.clone(),
    }
}

/// The year, month and day of `YYYY-MM-DD`.
fn day_parts(date: &str) -> Option<(&str, &str, &str)> {
    let mut parts = date.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    let digits = |part: &str, length: usize| {
        part.len() == length && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    (parts.next().is_none() && digits(y, 4) && digits(m, 2) && digits(d, 2)).then_some((y, m, d))
}

fn is_code(code: &str) -> bool {
    code.len() == 3 && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

/// Every unit's id, for the names `i18n` must have.
#[cfg(test)]
pub fn unit_ids() -> impl Iterator<Item = &'static str> {
    UNITS.iter().map(|unit| unit.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed rate for the tests: 1 € = 1,10 US$ = 6,00 R$ = 0,85 £.
    fn rates() -> Rates {
        Rates {
            date: "2026-10-30".to_owned(),
            rates: [
                ("EUR", 1.0),
                ("USD", 1.1),
                ("BRL", 6.0),
                ("GBP", 0.85),
                ("JPY", 160.0),
            ]
            .into_iter()
            .map(|(code, rate)| (code.to_owned(), rate))
            .collect(),
        }
    }

    /// The single conversion of `query`, as `(from, to, value)`.
    fn one(query: &str) -> Option<(String, String, f64)> {
        let found = convert(query, Some(&rates()));
        assert!(found.len() <= 1, "{query:?} gave {found:?}");
        found
            .into_iter()
            .next()
            .map(|c| (symbol(&c.from), symbol(&c.to), c.value))
    }

    fn value(query: &str) -> f64 {
        one(query)
            .unwrap_or_else(|| panic!("{query:?} gave nothing"))
            .2
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    fn assert_value(query: &str, expected: f64) {
        let got = value(query);
        assert!(close(got, expected), "{query:?}: {got} != {expected}");
    }

    fn no_names(_: &str, _: bool) -> Option<&'static str> {
        None
    }

    #[test]
    fn length() {
        assert_value("10 km em mi", 6.213_711_922_373_339);
        assert_value("1 mi em m", 1609.344);
        assert_value("12 in em cm", 30.48);
        assert_value("3 ft para in", 36.0);
        assert_value("1 yd em ft", 3.0);
        assert_value("1000 mm em m", 1.0);
        assert_value("1 milha nautica em km", 1.852);
        assert_value("10 pol em cm", 25.4);
    }

    #[test]
    fn mass() {
        assert_value("1 kg em lb", 2.204_622_621_848_776);
        assert_value("16 oz em lb", 1.0);
        assert_value("1 t em kg", 1000.0);
        assert_value("500 mg em g", 0.5);
        assert_value("1000 g to kg", 1.0);
    }

    #[test]
    fn temperature_is_not_proportional() {
        assert_value("30c em f", 86.0);
        assert_value("100 °C em F", 212.0);
        assert_value("-40 c em f", -40.0);
        assert_value("32 f em c", 0.0);
        assert_value("0 k em c", -273.15);
        assert_value("25 celsius em kelvin", 298.15);
        assert_value("98,6 fahrenheit em celsius", 37.0);
        assert_value("30°C em °F", 86.0);
    }

    #[test]
    fn volume_area_and_speed() {
        assert_value("1 gal em l", 3.785_411_784);
        assert_value("1 l em ml", 1000.0);
        assert_value("1 m³ em l", 1000.0);
        assert_value("1 fl oz em ml", 29.573_529_562_5);
        assert_value("2 xicaras em ml", 480.0);
        assert_value("1 ha em m²", 10_000.0);
        assert_value("1 km2 em ha", 100.0);
        assert_value("1 acre em m2", 4_046.856_422_4);
        assert_value("10 m2 em ft2", 107.639_104_167_097_2);
        assert_value("36 km/h em m/s", 10.0);
        assert_value("60 mph em km/h", 96.560_64);
        assert_value("10 nos em km/h", 18.52);
    }

    #[test]
    fn data_with_decimal_and_binary_units() {
        assert_value("5 gb em mb", 5000.0);
        assert_value("1 GiB em MiB", 1024.0);
        assert_value("1 gib em mb", 1_073.741_824);
        assert_value("2 tb em gb", 2000.0);
        assert_value("2048 kib em kb", 2097.152);
        assert_value("1 kb em b", 1000.0);
    }

    #[test]
    fn time() {
        assert_value("2 horas em minutos", 120.0);
        assert_value("1 dia em h", 24.0);
        assert_value("1 semana em dias", 7.0);
        assert_value("90 s em min", 1.5);
    }

    #[test]
    fn names_in_portuguese_and_english_without_case_or_accents() {
        assert_value("10 quilômetros em milhas", value("10 km em mi"));
        assert_value("10 Kilometers to Miles", value("10 km em mi"));
        assert_value("10 KM PARA MILHAS", value("10 km em mi"));
        assert_value("5 pés em metros", 1.524);
        assert_value("5 feet in meters", 1.524);
        assert_value("10 in in cm", 25.4);
        assert_value("3 polegadas para centímetros", 7.62);
        assert_value("2 hours to minutes", 120.0);
        assert_value("1 xícara em ml", 240.0);
    }

    #[test]
    fn a_comma_or_a_point_is_the_decimal_mark() {
        assert_value("1,5 km em m", 1500.0);
        assert_value("1.5 km em m", 1500.0);
        assert_value(",5 kg em g", 500.0);
        assert!(one("1,5,2 km em m").is_none(), "two marks");
        let shown = show(&convert("10 km em milhas", None)[0], ',', no_names, "");
        assert_eq!(shown.copied, "6,21");
        let shown = show(&convert("10 km em milhas", None)[0], '.', no_names, "");
        assert_eq!(shown.copied, "6.21");
    }

    #[test]
    fn units_of_different_quantities_do_not_convert() {
        for query in [
            "10 km em kg",
            "30c em m",
            "5 gb em horas",
            "1 l em m2",
            "10 kg em usd",
        ] {
            assert!(convert(query, Some(&rates())).is_empty(), "{query:?}");
        }
    }

    #[test]
    fn what_is_not_a_conversion_gives_nothing() {
        for query in [
            "",
            "2024",
            "chrome",
            "km",
            "km em mi",
            "10 km em",
            "10 xyz em km",
            "10 km em xyz",
            "2+2",
            "g 10 km",
            "3d",
            "3d viewer",
            "10 km em mi em m",
        ] {
            assert!(convert(query, Some(&rates())).is_empty(), "{query:?}");
        }
    }

    #[test]
    fn a_unit_alone_shows_its_common_conversions() {
        let found = convert("10 km", None);
        let to: Vec<String> = found.iter().map(|c| symbol(&c.to)).collect();
        assert_eq!(to, ["mi", "m", "nmi"]);
        assert!(found.len() <= MAX_COMMON);
        let to: Vec<String> = convert("30 c", None)
            .iter()
            .map(|c| symbol(&c.to))
            .collect();
        assert_eq!(to, ["°F", "K"]);
        assert_eq!(
            convert("5gb", None).len(),
            3,
            "a unit of letters glued is fine"
        );
        assert!(
            convert("30c", None).is_empty(),
            "one letter glued needs the em"
        );
        // Every unit has conversions of its own quantity.
        for unit in UNITS {
            assert!(!unit.common.is_empty(), "{}", unit.id);
            for id in unit.common {
                let target = unit_by_id(id).unwrap_or_else(|| panic!("{id}"));
                assert_eq!(target.quantity, unit.quantity, "{} → {id}", unit.id);
                assert_ne!(target.id, unit.id);
            }
        }
    }

    #[test]
    fn shown_with_names_and_the_amount_under() {
        let name = |id: &str, plural: bool| match (id, plural) {
            ("mi", true) => Some("milhas"),
            ("mi", false) => Some("milha"),
            _ => None,
        };
        let shown = show(&convert("10 km em milhas", None)[0], ',', name, "");
        assert_eq!(
            shown,
            Shown {
                title: "= 6,21 milhas".into(),
                subtitle: "10 km".into(),
                copied: "6,21".into(),
            }
        );
        let shown = show(&convert("1,609344 km em mi", None)[0], ',', name, "");
        assert_eq!(shown.title, "= 1 milha");
        assert_eq!(shown.subtitle, "1,609344 km");
        // Without a name, the symbol; a small value keeps its digits.
        let shown = show(&convert("1 mm em km", None)[0], ',', no_names, "");
        assert_eq!(shown.title, "= 0,000001 km");
        let shown = show(&convert("30c em f", None)[0], ',', no_names, "");
        assert_eq!(
            (shown.title.as_str(), shown.subtitle.as_str()),
            ("= 86 °F", "30 °C")
        );
    }

    #[test]
    fn currencies_with_a_fixed_rate() {
        assert_value("100 usd em brl", 100.0 / 1.1 * 6.0);
        assert_value("50 euros para reais", 300.0);
        assert_value("10 dólares em euros", 10.0 / 1.1);
        assert_value("R$ 60 em eur", 10.0);
        assert_value("r$60 em eur", 10.0);
        assert_value("US$ 11 em €", 10.0);
        assert_value("$11 em eur", 10.0);
        assert_value("€ 10 em £", 8.5);
        assert_value("100 € em r$", 600.0);
        assert_value("1 eur em jpy", 160.0);
        // `libra` is a pound of mass or of money, by what it meets.
        assert_eq!(one("10 libras em reais").map(|c| c.1), Some("BRL".into()));
        assert_eq!(one("10 libras em kg").map(|c| c.1), Some("kg".into()));
        let shown = show(
            &convert("100 usd em brl", Some(&rates()))[0],
            ',',
            no_names,
            "Cotação BCE de {d}/{m}/{y}",
        );
        assert_eq!(
            shown,
            Shown {
                title: "= 545,45 BRL".into(),
                subtitle: "100 USD · Cotação BCE de 30/10/2026".into(),
                copied: "545,45".into(),
            }
        );
        let shown = show(
            &convert("50 eur em brl", Some(&rates()))[0],
            ',',
            no_names,
            "",
        );
        assert_eq!(shown.copied, "300,00", "always two decimals");
        // A currency alone: the common ones, without itself.
        let to: Vec<String> = convert("100 usd", Some(&rates()))
            .iter()
            .map(|c| symbol(&c.to))
            .collect();
        assert_eq!(to, ["BRL", "EUR", "GBP"]);
        assert!(is_currency(&convert("100 usd", Some(&rates()))));
        assert!(!is_currency(&convert("10 km", None)));
        // A code the rates do not have.
        assert!(convert("100 xyz em brl", Some(&rates())).is_empty());
        assert!(convert("100 usd em cad", Some(&rates())).is_empty());
    }

    #[test]
    fn without_rates_a_currency_gives_nothing() {
        for query in [
            "100 usd em brl",
            "50 euros para reais",
            "R$ 10",
            "100 dolares",
            "10 libras em reais",
        ] {
            assert!(convert(query, None).is_empty(), "{query:?}");
        }
        // The units still convert.
        assert_eq!(convert("10 km em mi", None).len(), 1);
    }

    #[test]
    fn frankfurter_answer_is_read() {
        let body = br#"{"amount":1.0,"base":"EUR","date":"2026-10-30","rates":{"BRL":6.0692,"USD":1.1172}}"#;
        let read = parse_rates(body).expect("rates");
        assert_eq!(read.date, "2026-10-30");
        assert_eq!(read.rates.get("EUR"), Some(&1.0));
        assert_eq!(read.rates.get("BRL"), Some(&6.0692));
        for bad in [
            &br#"{"amount":1.0,"base":"USD","date":"2026-10-30","rates":{"BRL":5.4}}"#[..],
            br#"{"base":"EUR","date":"30/10/2026","rates":{"BRL":6.0}}"#,
            br#"{"base":"EUR","date":"2026-10-30","rates":{}}"#,
            br#"{"base":"EUR","date":"2026-10-30","rates":{"BRL":-1}}"#,
            br#"{"base":"EUR","date":"2026-10-30","rates":{"brl":6.0}}"#,
            b"<html>erro</html>",
        ] {
            assert!(
                parse_rates(bad).is_none(),
                "{}",
                String::from_utf8_lossy(bad)
            );
        }
    }

    #[test]
    fn the_cache_is_kept_a_day() {
        let cache = Cache {
            fetched: "2026-10-30".into(),
            rates: rates(),
        };
        let text = write_cache(&cache);
        assert_eq!(read_cache(&text), Some(cache.clone()));
        assert!(read_cache("{}").is_none());
        assert!(read_cache("não é json").is_none());
        assert!(is_fresh(Some(&cache), "2026-10-30"));
        assert!(!is_fresh(Some(&cache), "2026-10-31"), "a day later, stale");
        assert!(!is_fresh(None, "2026-10-30"));
    }

    #[test]
    fn a_failed_fetch_keeps_the_old_rates_or_none() {
        let old = Cache {
            fetched: "2026-10-29".into(),
            rates: rates(),
        };
        // No network: the stale rates still answer, with their own day.
        assert_eq!(
            after_fetch(Some(old.clone()), None, "2026-10-30"),
            Some(old)
        );
        // No network and nothing kept: the currencies give nothing.
        assert_eq!(after_fetch(None, None, "2026-10-30"), None);
        // A fetch replaces them, marked today.
        let new = after_fetch(None, Some(rates()), "2026-10-31").expect("cache");
        assert_eq!(new.fetched, "2026-10-31");
        assert_eq!(new.rates, rates());
    }

    #[test]
    fn every_name_is_one_unit_of_a_quantity() {
        let mut seen: Vec<(&str, Quantity)> = Vec::new();
        for unit in UNITS {
            for name in unit.names {
                assert_eq!(normalize(name), *name, "{name} is normalized");
                assert!(
                    !seen.contains(&(name, unit.quantity)),
                    "{name} twice in {:?}",
                    unit.quantity
                );
                seen.push((name, unit.quantity));
            }
        }
    }
}
