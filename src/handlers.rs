use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post, put};
use axum::{Json, Router};
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
        .route("/health", get(health))
        .route("/state", get(get_state))
        .route("/state", put(put_state))
        .route("/update", post(trigger_update))
        .with_state(server_state)
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
