use reqwest::Client;
use reqwest::redirect::Policy;
use tracing::debug;

use crate::models::{IpResponse, MamResponse, SeedboxResponse};

const IP_URL: &str = "https://t.myanonamouse.net/json/jsonIp.php";
const SEEDBOX_URL: &str = "https://t.myanonamouse.net/json/dynamicSeedbox.php";

pub struct MamClient {
    client: Client,
}

impl MamClient {
    pub fn new(user_agent: &str) -> Self {
        let client = Client::builder()
            .user_agent(user_agent)
            .redirect(Policy::none())
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }

    pub async fn fetch_ip(&self) -> reqwest::Result<IpResponse> {
        self.client.get(IP_URL).send().await?.json().await
    }

    pub async fn update_seedbox(&self, cookie: &str) -> reqwest::Result<MamResponse> {
        let resp = self
            .client
            .get(SEEDBOX_URL)
            .header("Cookie", format!("mam_id={cookie}"))
            .send()
            .await?;

        let status = resp.status().as_u16();
        let new_cookie = extract_mam_id(resp.headers());

        if let Some(ref c) = new_cookie {
            debug!("received rotated mam_id cookie: {}", &c[..8.min(c.len())]);
        }

        let body: Option<SeedboxResponse> = if resp.status().is_success() {
            resp.json().await.ok()
        } else {
            None
        };

        Ok(MamResponse {
            at: time::OffsetDateTime::now_utc(),
            http_status: status,
            cookie_sent: cookie.to_string(),
            cookie_received: new_cookie,
            body,
        })
    }
}

fn extract_mam_id(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|cookie_str| {
            cookie_str
                .split(';')
                .next()
                .and_then(|pair| pair.strip_prefix("mam_id="))
                .map(str::to_string)
        })
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue};

    use super::*;

    #[test]
    fn extract_single_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "set-cookie",
            HeaderValue::from_static("mam_id=abc123; Path=/"),
        );
        assert_eq!(extract_mam_id(&headers), Some("abc123".to_string()));
    }

    #[test]
    fn extract_from_multiple_headers() {
        let mut headers = HeaderMap::new();
        headers.append("set-cookie", HeaderValue::from_static("other=xyz; Path=/"));
        headers.append(
            "set-cookie",
            HeaderValue::from_static("mam_id=def456; Path=/; HttpOnly"),
        );
        assert_eq!(extract_mam_id(&headers), Some("def456".to_string()));
    }

    #[test]
    fn no_mam_id_present() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "set-cookie",
            HeaderValue::from_static("session=xyz; Path=/"),
        );
        assert_eq!(extract_mam_id(&headers), None);
    }

    #[test]
    fn empty_headers() {
        let headers = HeaderMap::new();
        assert_eq!(extract_mam_id(&headers), None);
    }
}
