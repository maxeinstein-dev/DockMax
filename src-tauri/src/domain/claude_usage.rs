//! The Uso do Claude (spec 0017 F5): the plan's limits as the Claude apps
//! show them, the five hours' and the week's, each with the share used and
//! when it starts over. Pure: `windows/claude_usage.rs` reads the Claude
//! Code's credentials and asks Anthropic.

use serde::{Deserialize, Serialize};

/// One limit: the share used, 0–100, and when it starts over (Unix
/// seconds), if Anthropic said.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    pub percent: f64,
    pub resets_at: Option<i64>,
}

/// The two limits; `None` for one Anthropic didn't send.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub five_hour: Option<Limit>,
    pub seven_day: Option<Limit>,
}

#[derive(Deserialize)]
struct RawLimit {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct RawUsage {
    five_hour: Option<RawLimit>,
    seven_day: Option<RawLimit>,
}

fn limit_of(raw: Option<RawLimit>) -> Option<Limit> {
    let raw = raw?;
    Some(Limit {
        percent: raw.utilization?.clamp(0.0, 100.0),
        resets_at: raw
            .resets_at
            .and_then(|text| text.parse::<jiff::Timestamp>().ok())
            .map(|at| at.as_second()),
    })
}

/// The usage in Anthropic's answer (`/api/oauth/usage`); `None` when it is
/// not one.
pub fn parse_usage(json: &[u8]) -> Option<Usage> {
    let raw: RawUsage = serde_json::from_slice(json).ok()?;
    let usage = Usage {
        five_hour: limit_of(raw.five_hour),
        seven_day: limit_of(raw.seven_day),
    };
    (usage.five_hour.is_some() || usage.seven_day.is_some()).then_some(usage)
}

/// The Claude Code's sign-in, as its credentials file keeps it.
pub struct Credentials {
    pub access_token: String,
    /// When the token expires, in Unix milliseconds, if the file says.
    pub expires_at: Option<i64>,
}

impl std::fmt::Debug for Credentials {
    // Never the token, wherever this is printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawOauth {
    access_token: Option<String>,
    expires_at: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCredentials {
    claude_ai_oauth: Option<RawOauth>,
}

/// The sign-in in the Claude Code's credentials file (`.credentials.json`);
/// `None` without one.
pub fn parse_credentials(json: &[u8]) -> Option<Credentials> {
    let raw: RawCredentials = serde_json::from_slice(json).ok()?;
    let oauth = raw.claude_ai_oauth?;
    let access_token = oauth
        .access_token
        .filter(|token| !token.trim().is_empty())?;
    Some(Credentials {
        access_token,
        expires_at: oauth.expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_five_hours_and_the_week() {
        let json = br#"{
            "five_hour": { "utilization": 37.0, "resets_at": "2026-10-29T18:00:00.000000+00:00" },
            "seven_day": { "utilization": 12.5, "resets_at": "2026-11-02T13:00:00+00:00" },
            "seven_day_opus": null
        }"#;
        let usage = parse_usage(json).unwrap();
        let five = usage.five_hour.unwrap();
        assert_eq!(five.percent, 37.0);
        assert_eq!(
            five.resets_at,
            Some(
                "2026-10-29T18:00:00Z"
                    .parse::<jiff::Timestamp>()
                    .unwrap()
                    .as_second()
            )
        );
        assert_eq!(usage.seven_day.unwrap().percent, 12.5);
    }

    #[test]
    fn a_limit_without_a_reset_or_an_odd_answer() {
        let usage = parse_usage(br#"{"five_hour":{"utilization":140,"resets_at":null}}"#).unwrap();
        assert_eq!(
            usage.five_hour,
            Some(Limit {
                percent: 100.0,
                resets_at: None
            })
        );
        assert_eq!(usage.seven_day, None);
        assert!(parse_usage(br#"{"error":"unauthorized"}"#).is_none());
        assert!(parse_usage(b"not json").is_none());
    }

    #[test]
    fn it_reads_the_sign_in_and_never_prints_the_token() {
        let json = br#"{"claudeAiOauth":{"accessToken":"not-a-real-token","refreshToken":"x","expiresAt":1761700000000,"scopes":["user:inference"]}}"#;
        let credentials = parse_credentials(json).unwrap();
        assert_eq!(credentials.access_token, "not-a-real-token");
        assert_eq!(credentials.expires_at, Some(1_761_700_000_000));
        assert!(!format!("{credentials:?}").contains("not-a-real-token"));
        assert!(parse_credentials(br#"{"claudeAiOauth":{"accessToken":"  "}}"#).is_none());
        assert!(parse_credentials(b"{}").is_none());
    }
}
