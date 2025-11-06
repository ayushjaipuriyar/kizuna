//! Web Server Implementation
//! 
//! HTTP/WebSocket server for browser client communication.

use crate::browser_support::{BrowserResult, BrowserSupportError, discovery::BrowserDiscovery};
use crate::browser_support::types::*;
use crate::browser_support::api::handlers::APIHandlers;
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::{Html, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

/// Web server for browser API
pub struct WebServer {
    discovery_manager: Arc<BrowserDiscovery>,
    shutdown_signal: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Server state shared across handlers
#[derive(Clone)]
pub struct ServerState {
    pub handlers: Arc<APIHandlers>,
    pub discovery_manager: Arc<BrowserDiscovery>,
}

/// Query parameters for connection setup
#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub setup_id: Uuid,
}

/// Request body for browser connection
#[derive(Debug, Deserialize)]
pub struct BrowserConnectRequest {
    pub browser_info: BrowserInfo,
}

impl WebServer {
    /// Create a new web server
    pub fn new(discovery_manager: Arc<BrowserDiscovery>) -> Self {
        Self {
            discovery_manager,
            shutdown_signal: None,
        }
    }
    
    /// Start the web server
    pub async fn start(&mut self, port: u16) -> BrowserResult<()> {
        let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()
            .map_err(|e| BrowserSupportError::ConfigurationError {
                parameter: "address".to_string(),
                issue: format!("Invalid address format: {}", e),
            })?;

        // Initialize discovery manager with server address
        // Note: We need to make discovery_manager mutable, but it's Arc<>
        // For now, we'll assume it's initialized elsewhere
        
        let handlers = Arc::new(APIHandlers::new(self.discovery_manager.clone()));
        let state = ServerState {
            handlers,
            discovery_manager: self.discovery_manager.clone(),
        };

        let app = create_router(state);

        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| BrowserSupportError::NetworkError {
                details: format!("Failed to bind to {}: {}", addr, e),
            })?;

        println!("Browser API server listening on {}", addr);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        self.shutdown_signal = Some(shutdown_tx);

        // Start the server
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .unwrap();
        });

        Ok(())
    }
    
    /// Shutdown the web server
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        if let Some(signal) = self.shutdown_signal.take() {
            let _ = signal.send(());
        }
        Ok(())
    }
}

/// Create the Axum router with all endpoints
fn create_router(state: ServerState) -> Router {
    Router::new()
        // Discovery and connection setup endpoints
        .route("/api/setup/create", post(create_connection_setup))
        .route("/api/setup/:setup_id", get(get_connection_setup))
        .route("/api/setup/:setup_id/qr", get(get_qr_code))
        .route("/api/connect", post(connect_browser))
        
        // Peer discovery endpoints
        .route("/api/peers/discover", get(discover_peers))
        .route("/api/peers/local", get(get_local_peer))
        
        // Connection status endpoints
        .route("/api/connections", get(get_all_connections))
        .route("/api/connections/:session_id", get(get_connection_status))
        
        // Browser client interface
        .route("/connect", get(browser_connect_page))
        .route("/", get(index_page))
        
        // WebSocket endpoint for signaling
        .route("/ws", get(websocket_handler))
        
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Create a new connection setup
async fn create_connection_setup(
    State(state): State<ServerState>,
) -> Result<Json<Value>, StatusCode> {
    match state.handlers.handle_create_connection_setup().await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get connection setup by ID
async fn get_connection_setup(
    State(state): State<ServerState>,
    Path(setup_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    match state.handlers.handle_get_connection_setup(setup_id).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Get QR code for connection setup
async fn get_qr_code(
    State(state): State<ServerState>,
    Path(setup_id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    match state.handlers.handle_generate_qr_code(setup_id).await {
        Ok(svg) => {
            let mut headers = HeaderMap::new();
            headers.insert("content-type", "image/svg+xml".parse().unwrap());
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "image/svg+xml")
                .body(svg.into())
                .unwrap())
        },
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Connect browser client
async fn connect_browser(
    State(state): State<ServerState>,
    Query(params): Query<ConnectQuery>,
    Json(request): Json<BrowserConnectRequest>,
) -> Result<Json<Value>, StatusCode> {
    match state.handlers.handle_create_browser_connection_info(
        params.setup_id,
        request.browser_info,
    ).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Discover available peers
async fn discover_peers(
    State(state): State<ServerState>,
) -> Result<Json<Value>, StatusCode> {
    match state.handlers.handle_discover_peers().await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get local peer information
async fn get_local_peer(
    State(state): State<ServerState>,
) -> Result<Json<Value>, StatusCode> {
    let local_peer = state.discovery_manager.get_local_peer_info().await;
    Ok(Json(serde_json::to_value(local_peer).unwrap()))
}

/// Get all connection statuses
async fn get_all_connections(
    State(state): State<ServerState>,
) -> Result<Json<Value>, StatusCode> {
    match state.handlers.handle_get_all_connection_statuses().await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Get specific connection status
async fn get_connection_status(
    State(state): State<ServerState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    match state.handlers.handle_get_connection_status(session_id).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Browser connection page
async fn browser_connect_page() -> Result<Response, StatusCode> {
    let html = include_str!("../static/connect.html");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html")
        .body(html.to_string().into())
        .unwrap())
}

/// Index page
async fn index_page() -> Result<Response, StatusCode> {
    let html = include_str!("../static/index.html");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html")
        .body(html.to_string().into())
        .unwrap())
}

/// WebSocket handler for signaling
async fn websocket_handler() -> StatusCode {
    // TODO: Implement WebSocket upgrade for signaling
    // This will handle WebRTC signaling messages between browser and peer
    StatusCode::NOT_IMPLEMENTED
}