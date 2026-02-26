use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;
use tracing::info;

use crate::error::AppError;
use crate::mam::MamClient;
use crate::models::PersistentState;
use crate::scheduler;
use crate::state::SharedState;

#[derive(Clone)]
pub struct ServerState {
    pub shared: SharedState,
    pub client: Arc<MamClient>,
}

pub fn router(shared: SharedState, client: Arc<MamClient>) -> Router {
    let server_state = ServerState { shared, client };
    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/state", get(get_state))
        .route("/state", put(put_state))
        .route("/update", post(trigger_update))
        .with_state(server_state)
}

#[allow(clippy::too_many_lines)]
async fn index(State(s): State<ServerState>) -> Markup {
    let state = s.shared.read().await;
    let persistent = &state.persistent;

    let cookie_set = persistent.cookie.is_some();
    let masked_cookie = persistent.cookie.as_deref().map(mask_cookie);

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "mouser" }
                style {
                    (maud::PreEscaped(r#"
                        *, *::before, *::after { box-sizing: border-box; }
                        body {
                            font-family: system-ui, -apple-system, sans-serif;
                            max-width: 40rem;
                            margin: 2rem auto;
                            padding: 0 1rem;
                            color: #1a1a1a;
                            background: #fafafa;
                            line-height: 1.5;
                        }
                        h1 { font-size: 1.5rem; margin-bottom: 1.5rem; }
                        section { margin-bottom: 2rem; }
                        h2 { font-size: 1.1rem; margin-bottom: 0.5rem; color: #444; }
                        .field { margin-bottom: 0.5rem; }
                        .label { font-weight: 600; }
                        .mono { font-family: ui-monospace, monospace; font-size: 0.9em; overflow-wrap: break-word; }
                        .muted { color: #888; }
                        input[type="text"] {
                            width: 100%;
                            padding: 0.5rem;
                            border: 1px solid #ccc;
                            border-radius: 4px;
                            font-family: ui-monospace, monospace;
                            font-size: 0.9em;
                            margin-bottom: 0.5rem;
                        }
                        input[type="text"]:focus { outline: 2px solid #2563eb; border-color: transparent; }
                        button {
                            padding: 0.5rem 1rem;
                            border: none;
                            border-radius: 4px;
                            cursor: pointer;
                            font-size: 0.9rem;
                            font-weight: 500;
                        }
                        .btn-primary { background: #2563eb; color: white; }
                        .btn-primary:hover { background: #1d4ed8; }
                        .btn-secondary { background: #e5e7eb; color: #1a1a1a; }
                        .btn-secondary:hover { background: #d1d5db; }
                        .btn-row { display: flex; gap: 0.5rem; }
                        .status { padding: 0.75rem; border-radius: 4px; background: #f3f4f6; margin-top: 0.5rem; font-size: 0.9em; }
                        .status.ok { background: #dcfce7; }
                        .status.warn { background: #fef9c3; }
                        .status.error { background: #fee2e2; }
                        #result { margin-top: 1rem; }
                    "#))
                }
            }
            body {
                h1 { "mouser" }

                section {
                    h2 { "Cookie" }
                    @if cookie_set {
                        div.field {
                            span.label { "Current: " }
                            span.mono { (masked_cookie.unwrap_or_default()) }
                        }
                    } @else {
                        div.field.muted { "No cookie set" }
                    }
                    input #cookie-input type="text" placeholder="mam_id cookie value" {}
                    div.btn-row {
                        button.btn-primary #set-cookie-btn { "Set Cookie" }
                    }
                }

                section {
                    h2 { "Update" }
                    div.btn-row {
                        button.btn-secondary #trigger-btn { "Trigger Update" }
                    }
                }

                section {
                    h2 { "Last Update" }
                    @if let Some(ref update) = persistent.last_update {
                        div.field {
                            span.label { "At: " }
                            time.mono.local-time datetime=(update.at.to_string()) { (update.at.to_string()) }
                        }
                        div.field {
                            span.label { "Reason: " }
                            span { (format!("{:?}", update.reason)) }
                        }
                        div.field {
                            span.label { "MAM updated: " }
                            span { (if update.mam_updated { "yes" } else { "no" }) }
                        }
                    } @else {
                        div.field.muted { "No update yet" }
                    }

                    @if let Some(ref resp) = persistent.last_mam_response {
                        div.field {
                            span.label { "Last MAM response: " }
                            span { (resp.http_status) " at " }
                            time.mono.local-time datetime=(resp.at.to_string()) { (resp.at.to_string()) }
                        }
                    }
                }

                div #result {}

                script {
                    (maud::PreEscaped(r"
                        const $result = document.getElementById('result');
                        function showResult(cls, msg) {
                            $result.className = 'status ' + cls;
                            $result.textContent = msg;
                        }

                        document.getElementById('set-cookie-btn').addEventListener('click', async () => {
                            const cookie = document.getElementById('cookie-input').value.trim();
                            if (!cookie) { showResult('warn', 'Please enter a cookie value'); return; }
                            try {
                                const res = await fetch('/state', {
                                    method: 'PUT',
                                    headers: { 'Content-Type': 'application/json' },
                                    body: JSON.stringify({ cookie }),
                                });
                                if (res.ok) {
                                    showResult('ok', 'Cookie updated');
                                    setTimeout(() => location.reload(), 800);
                                } else {
                                    showResult('error', 'Error: ' + res.status);
                                }
                            } catch (e) {
                                showResult('error', 'Request failed: ' + e.message);
                            }
                        });

                        document.getElementById('trigger-btn').addEventListener('click', async () => {
                            showResult('', 'Updating…');
                            try {
                                const res = await fetch('/update', { method: 'POST' });
                                if (res.ok) {
                                    showResult('ok', 'Update triggered');
                                    setTimeout(() => location.reload(), 800);
                                } else {
                                    showResult('error', 'Error: ' + res.status);
                                }
                            } catch (e) {
                                showResult('error', 'Request failed: ' + e.message);
                            }
                        });

                        document.querySelectorAll('.local-time').forEach(el => {
                            const d = new Date(el.getAttribute('datetime'));
                            if (!isNaN(d)) el.textContent = d.toLocaleString();
                        });
                    "))
                }
            }
        }
    }
}

fn mask_cookie(cookie: &str) -> String {
    let len = cookie.len();
    if len <= 8 {
        "*".repeat(len)
    } else {
        let visible = &cookie[..4];
        format!("{visible}{}", "*".repeat(len - 4))
    }
}

async fn health(State(s): State<ServerState>) -> Result<Json<serde_json::Value>, AppError> {
    let state = s.shared.read().await;

    if state.persistent.cookie.is_none() {
        return Err(AppError::NoCookie);
    }

    if let Some(ref resp) = state.persistent.last_mam_response {
        if !resp.http_status.to_string().starts_with('2') {
            return Err(AppError::Internal("last MAM response was an error".into()));
        }
    } else {
        return Err(AppError::Internal("no successful update yet".into()));
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn get_state(State(s): State<ServerState>) -> Json<PersistentState> {
    let state = s.shared.read().await;
    Json(state.persistent.clone())
}

#[derive(Deserialize)]
struct CookieUpdate {
    cookie: String,
}

async fn put_state(
    State(s): State<ServerState>,
    Json(body): Json<CookieUpdate>,
) -> Result<Json<PersistentState>, AppError> {
    let mut state = s.shared.write().await;
    state.persistent.cookie = Some(body.cookie);
    state
        .save()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    info!("cookie updated via API");
    Ok(Json(state.persistent.clone()))
}

async fn trigger_update(State(s): State<ServerState>) -> Result<Json<PersistentState>, AppError> {
    scheduler::run_tick(&s.shared, &s.client).await;
    let state = s.shared.read().await;
    Ok(Json(state.persistent.clone()))
}
