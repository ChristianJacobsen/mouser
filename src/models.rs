use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Response from `GET /json/jsonIp.php`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpResponse {
    pub ip: String,
    #[serde(rename = "ASN")]
    pub asn: u32,
    #[serde(rename = "AS")]
    pub as_name: String,
}

/// Response from `GET /json/dynamicSeedbox.php`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedboxResponse {
    #[serde(rename = "Success")]
    pub success: bool,
    pub msg: String,
    pub ip: String,
    #[serde(rename = "ASN")]
    pub asn: u32,
    #[serde(rename = "AS")]
    pub as_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MamResponse {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub http_status: u16,
    pub cookie_sent: String,
    pub cookie_received: Option<String>,
    pub body: Option<SeedboxResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastUpdate {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub mam_updated: bool,
    pub reason: UpdateReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateReason {
    NoPreviousResponse,
    PreviousHttpError,
    IpChanged,
    AsnChanged,
    Stale,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersistentState {
    pub cookie: Option<String>,
    pub last_mam_response: Option<MamResponse>,
    pub last_update: Option<LastUpdate>,
}
