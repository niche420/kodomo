use std::sync::{Arc, Mutex};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use kd_shared::profile::GameProfile;
use crate::profile::{delete_profile, list_profiles, load_profile, save_profile_named};
use crate::state::AppState;

pub type SharedState = Arc<Mutex<AppState>>;

pub async fn serve(state: SharedState) {
    let http_port = state.lock().unwrap().persistent.network.http_port;

    let app = Router::new()
        .route("/games", get(get_games))
        .route("/games/{title}/profiles", get(get_profiles))
        .route("/games/{title}/profiles/{name}", get(get_profile))
        .route("/games/{title}/profiles/{name}", put(put_profile))
        .route("/games/{title}/profiles/{name}", delete(del_profile))
        .route("/games/{title}/active", get(get_active))
        .route("/games/{title}/active", put(put_active))
        .route("/stream", post(start_stream))
        .route("/stream", delete(stop_stream))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", http_port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_games(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.lock().unwrap();
    let games: Vec<serde_json::Value> = state.persistent.games.iter().map(|g| json!({
        "title": g.metadata.title,
        "active_profile": g.active_profile,
    })).collect();
    Json(games)
}

async fn get_profiles(
    State(_): State<SharedState>,
    Path(title): Path<String>,
) -> impl IntoResponse {
    Json(list_profiles(&title))
}

async fn get_profile(
    State(_): State<SharedState>,
    Path((title, name)): Path<(String, String)>,
) -> impl IntoResponse {
    match load_profile(&title, &name) {
        Some(p) => Json(serde_json::to_value(p).unwrap()).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn put_profile(
    State(_): State<SharedState>,
    Path((title, name)): Path<(String, String)>,
    Json(profile): Json<GameProfile>,
) -> impl IntoResponse {
    match save_profile_named(&title, &name, &profile) {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn del_profile(
    State(_): State<SharedState>,
    Path((title, name)): Path<(String, String)>,
) -> impl IntoResponse {
    delete_profile(&title, &name);
    StatusCode::OK
}

async fn get_active(
    State(state): State<SharedState>,
    Path(title): Path<String>,
) -> impl IntoResponse {
    let state = state.lock().unwrap();
    match state.persistent.games.iter().find(|g| g.metadata.title == title) {
        Some(game) => Json(json!({ "active": game.active_profile })).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn put_active(
    State(state): State<SharedState>,
    Path(title): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = match body.get("active").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return StatusCode::BAD_REQUEST,
    };
    let mut state = state.lock().unwrap();
    match state.game_mut(&title) {
        Some(game) => { game.active_profile = Some(name); StatusCode::OK }
        None => StatusCode::NOT_FOUND,
    }
}

#[derive(Deserialize)]
struct StreamRequest {
    game: String,
}

#[derive(serde::Serialize)]
struct StreamResponse {
    token: String,
    handshake_port: u16,
}

async fn start_stream(
    State(state): State<SharedState>,
    Json(body): Json<StreamRequest>,
) -> impl IntoResponse {
    let mut state = state.lock().unwrap();
    if let Some(game) = state.game_clone(&body.game) {
        let token = state.new_token();
        let handshake_port = state.persistent.network.handshake_port;
        state.start_session(game);
        return Json(StreamResponse { token: token.to_string(), handshake_port }).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

async fn stop_stream(
    State(state): State<SharedState>,
) -> impl IntoResponse {
    let mut state = state.lock().unwrap();
    if state.is_streaming() {
        state.stop_session();
        return StatusCode::OK
    }

    StatusCode::NOT_FOUND
}