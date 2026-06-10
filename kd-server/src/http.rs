use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
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
use crate::network::HandshakeListener;
use crate::profile::{delete_profile, list_profiles, load_profile, save_profile_named};
use crate::state::{AppState, SessionState};
use crate::{network, ui};
use crate::ui::AppEvent;
use crate::ui::screen::ScreenType;

pub type SharedState = Arc<Mutex<AppState>>;

pub async fn serve(state: SharedState) {
    let app = Router::new()
        .route("/games", get(get_games))
        .route("/games/{title}/profiles", get(get_profiles))
        .route("/games/{title}/profiles/{name}", get(get_profile))
        .route("/games/{title}/profiles/{name}", put(put_profile))
        .route("/games/{title}/profiles/{name}", delete(del_profile))
        .route("/games/{title}/active", get(get_active))
        .route("/games/{title}/active", put(put_active))
        .route("/stream", post(post_stream))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7000").await.unwrap();
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

async fn post_stream(
    State(state): State<SharedState>,
    Json(body): Json<StreamRequest>,
) -> impl IntoResponse {
    let mut state = state.lock().unwrap();

    if !state.persistent.games.iter().any(|g| g.metadata.title == body.game) {
        return (StatusCode::NOT_FOUND, Json(json!({ "error": "Game not found" }))).into_response();
    }

    let token = Uuid::new_v4().to_string();
    let handshake_port = state.persistent.network.handshake_port;
    let input_port = state.persistent.network.input_port;

    // Store partial session
    state.session = Some(SessionState {
        token: token.clone(),
        client_ip: String::new(),
        game_title: body.game.clone(),
    });
    state.selected_game = Some(body.game);

    // Start listening BEFORE sending the response so the client
    // doesn't try to connect before we're ready
    state.push_event(AppEvent::ScreenTransition(ScreenType::Connect));

    Json(json!({
        "token": token,
        "handshake_port": handshake_port,
        "input_port": input_port,
    })).into_response()
}