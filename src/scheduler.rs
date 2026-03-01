use std::sync::Arc;
use std::time::Duration;

use time::OffsetDateTime;
use tracing::{error, info, warn};

use crate::mam::MamClient;
use crate::models::{LastUpdate, UpdateReason};
use crate::state::SharedState;

fn should_update(
    state: &crate::models::PersistentState,
    stale_threshold: u64,
) -> Option<UpdateReason> {
    let Some(ref resp) = state.last_mam_response else {
        return Some(UpdateReason::NoPreviousResponse);
    };

    if !resp.http_status.to_string().starts_with('2') {
        return Some(UpdateReason::PreviousHttpError);
    }

    if resp.body.is_none() {
        return Some(UpdateReason::PreviousHttpError);
    }

    Some(UpdateReason::Stale).filter(|_| {
        let age = (OffsetDateTime::now_utc() - resp.at)
            .whole_seconds()
            .unsigned_abs();
        age >= stale_threshold
    })
}

pub async fn run_tick(shared: &SharedState, client: &MamClient) {
    let (cookie, check_reason) = {
        let state = shared.read().await;
        let reason = should_update(&state.persistent, state.config.stale_threshold);
        let cookie = state.persistent.cookie.clone();
        (cookie, reason)
    };

    let Some(reason) = check_reason else {
        info!("no update needed");
        return;
    };

    let Some(cookie) = cookie else {
        warn!("update needed ({reason:?}) but no cookie configured");
        return;
    };

    info!(?reason, "update needed, fetching current IP");

    let ip = match client.fetch_ip().await {
        Ok(ip) => ip,
        Err(e) => {
            error!("failed to fetch IP: {e}");
            return;
        }
    };

    // Refine the reason based on what specifically changed.
    let update_reason = {
        let state = shared.read().await;
        let prev = state
            .persistent
            .last_mam_response
            .as_ref()
            .and_then(|r| r.body.as_ref());
        match prev {
            Some(p) if p.ip != ip.ip => UpdateReason::IpChanged,
            Some(p) if p.asn != ip.asn => UpdateReason::AsnChanged,
            _ => reason,
        }
    };

    // Always call MAM when should_update said so — even if the IP hasn't
    // changed, the stale call keeps the session alive and accepts cookie
    // rotation.
    info!(ip = %ip.ip, asn = ip.asn, ?update_reason, "reporting to MAM");

    match client.update_seedbox(&cookie).await {
        Ok(mam_resp) => {
            let mut state = shared.write().await;
            if let Some(ref new_cookie) = mam_resp.cookie_received {
                state.persistent.cookie = Some(new_cookie.clone());
            }
            state.persistent.last_mam_response = Some(mam_resp);
            state.persistent.last_update = Some(LastUpdate {
                at: OffsetDateTime::now_utc(),
                mam_updated: true,
                reason: update_reason,
            });
            if let Err(e) = state.save() {
                error!("failed to save state: {e}");
            }
        }
        Err(e) => {
            error!("failed to update MAM: {e}");
        }
    }
}

pub async fn run_scheduler(shared: SharedState, client: Arc<MamClient>) {
    let interval = {
        let state = shared.read().await;
        Duration::from_secs(state.config.check_interval)
    };

    let mut ticker = tokio::time::interval(interval);

    loop {
        ticker.tick().await;
        run_tick(&shared, &client).await;
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use crate::models::{MamResponse, PersistentState, SeedboxResponse};

    use super::*;

    fn make_response(age_secs: i64, ip: &str, asn: u32, status: u16) -> PersistentState {
        PersistentState {
            cookie: Some("test".to_string()),
            last_mam_response: Some(MamResponse {
                at: OffsetDateTime::now_utc() - time::Duration::seconds(age_secs),
                http_status: status,
                cookie_sent: "test".to_string(),
                cookie_received: None,
                body: if status == 200 {
                    Some(SeedboxResponse {
                        success: true,
                        msg: "ok".to_string(),
                        ip: ip.to_string(),
                        asn,
                        as_name: "AS Test".to_string(),
                    })
                } else {
                    None
                },
            }),
            last_update: None,
        }
    }

    #[test]
    fn no_previous_response() {
        let state = PersistentState::default();
        let result = should_update(&state, 86400);
        assert!(matches!(result, Some(UpdateReason::NoPreviousResponse)));
    }

    #[test]
    fn previous_http_error() {
        let state = make_response(60, "1.2.3.4", 1234, 403);
        let result = should_update(&state, 86400);
        assert!(matches!(result, Some(UpdateReason::PreviousHttpError)));
    }

    #[test]
    fn stale_response() {
        let state = make_response(90000, "1.2.3.4", 1234, 200);
        let result = should_update(&state, 86400);
        assert!(matches!(result, Some(UpdateReason::Stale)));
    }

    #[test]
    fn no_update_needed() {
        let state = make_response(60, "1.2.3.4", 1234, 200);
        let result = should_update(&state, 86400);
        assert!(result.is_none());
    }
}
