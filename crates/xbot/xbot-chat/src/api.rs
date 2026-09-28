//! The JSON API behind `xbot://localhost/api/*`, backed by the XBot daemon on
//! the session D-Bus and by the user's `config.toml`.

use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::sync::OnceCell;
use xbot_core::{Config, BUS_NAME, DAEMON_INTERFACE, OBJECT_PATH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Status,
    Incidents,
    GetConfig,
    PutConfig,
    SendChat,
    MethodNotAllowed,
    NotFound,
}

pub fn route(method: &str, path: &str) -> Route {
    let route = match path {
        "/api/status" => (Route::Status, "GET"),
        "/api/incidents" => (Route::Incidents, "GET"),
        "/api/config" if method == "PUT" => (Route::PutConfig, "PUT"),
        "/api/config" => (Route::GetConfig, "GET"),
        "/api/chat" => (Route::SendChat, "POST"),
        _ => return Route::NotFound,
    };
    if method == route.1 {
        route.0
    } else {
        Route::MethodNotAllowed
    }
}

#[derive(Debug, PartialEq)]
pub struct Reply {
    pub status: u16,
    pub body: Value,
}

impl Reply {
    fn ok(body: Value) -> Self {
        Self { status: 200, body }
    }

    fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            body: json!({ "error": message.into() }),
        }
    }
}

/// How a daemon call failed, as the UI needs to present it.
#[derive(Debug, PartialEq)]
pub enum DaemonError {
    /// No daemon on the bus, or no session bus at all.
    Offline(String),
    /// The daemon is running but this method belongs to a later phase.
    Unavailable(String),
    Failed(String),
}

impl DaemonError {
    pub fn from_error_name(name: &str, message: Option<String>) -> Self {
        let message = message.unwrap_or_else(|| name.to_string());
        match name {
            "org.freedesktop.DBus.Error.NotSupported" => Self::Unavailable(message),
            "org.freedesktop.DBus.Error.ServiceUnknown"
            | "org.freedesktop.DBus.Error.NameHasNoOwner"
            | "org.freedesktop.DBus.Error.NoReply"
            | "org.freedesktop.DBus.Error.Spawn.ChildExited"
            | "org.freedesktop.DBus.Error.Spawn.ExecFailed" => Self::Offline(message),
            _ => Self::Failed(message),
        }
    }

    fn from_zbus(err: zbus::Error) -> Self {
        match err {
            zbus::Error::MethodError(name, message, _) => {
                Self::from_error_name(name.as_str(), message)
            }
            zbus::Error::InputOutput(err) => Self::Offline(err.to_string()),
            zbus::Error::Address(message) => Self::Offline(message),
            other => Self::Failed(other.to_string()),
        }
    }

    /// The reply for an endpoint whose daemon call failed.
    pub fn reply(self) -> Reply {
        match self {
            Self::Offline(message) => Reply::ok(json!({ "state": "offline", "message": message })),
            Self::Unavailable(message) => {
                Reply::ok(json!({ "state": "unavailable", "message": message }))
            }
            Self::Failed(message) => Reply::error(502, message),
        }
    }
}

#[derive(Deserialize)]
struct ChatRequest {
    text: String,
}

pub struct Api {
    config_path: PathBuf,
    connection: OnceCell<zbus::Connection>,
}

/// Settings must arrive as a JSON object; serde would otherwise accept `[]`
/// as an all-default configuration.
fn parse_config(body: &[u8]) -> Result<Config, String> {
    let value: Value = serde_json::from_slice(body).map_err(|err| err.to_string())?;
    if !value.is_object() {
        return Err("expected a JSON object".into());
    }
    serde_json::from_value(value).map_err(|err| err.to_string())
}

impl Api {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            connection: OnceCell::new(),
        }
    }

    pub async fn handle(&self, method: &str, path: &str, body: &[u8]) -> Reply {
        match route(method, path) {
            Route::Status => match self.call::<_, String>("Status", &()).await {
                Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                    Ok(status) => Reply::ok(json!({ "state": "connected", "status": status })),
                    Err(err) => Reply::error(502, format!("invalid status from XBot: {err}")),
                },
                Err(err) => err.reply(),
            },
            Route::Incidents => match self.call::<_, String>("ListIncidents", &("{}",)).await {
                Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                    Ok(incidents) => {
                        Reply::ok(json!({ "state": "available", "incidents": incidents }))
                    }
                    Err(err) => Reply::error(502, format!("invalid incidents from XBot: {err}")),
                },
                Err(err) => err.reply(),
            },
            Route::GetConfig => match Config::load_from(self.config_path.clone()) {
                Ok(config) => Reply::ok(json!({ "config": config })),
                Err(err) => Reply::error(500, err.to_string()),
            },
            Route::PutConfig => match parse_config(body) {
                Ok(config) => match config.save_to(self.config_path.clone()) {
                    Ok(()) => Reply::ok(json!({ "config": config })),
                    Err(err) => Reply::error(500, err.to_string()),
                },
                Err(err) => Reply::error(400, format!("invalid settings: {err}")),
            },
            Route::SendChat => match serde_json::from_slice::<ChatRequest>(body) {
                Ok(request) if !request.text.trim().is_empty() => self.send_chat(request).await,
                Ok(_) => Reply::error(400, "message is empty"),
                Err(err) => Reply::error(400, format!("invalid message: {err}")),
            },
            Route::MethodNotAllowed => Reply::error(405, "method not allowed"),
            Route::NotFound => Reply::error(404, "not found"),
        }
    }

    async fn send_chat(&self, request: ChatRequest) -> Reply {
        let origin = json!({ "kind": "chat", "client": "xbot-chat" }).to_string();
        let session_id = match self
            .call::<_, String>("StartSession", &(origin.as_str(),))
            .await
        {
            Ok(id) => id,
            Err(err) => return err.reply(),
        };
        let message = json!({ "role": "user", "text": request.text }).to_string();
        match self
            .call::<_, ()>("SendMessage", &(session_id.as_str(), message.as_str()))
            .await
        {
            Ok(()) => Reply::ok(json!({ "state": "sent", "session_id": session_id })),
            Err(err) => err.reply(),
        }
    }

    async fn call<B, R>(&self, method: &str, body: &B) -> Result<R, DaemonError>
    where
        B: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let connection = self
            .connection
            .get_or_try_init(zbus::Connection::session)
            .await
            .map_err(DaemonError::from_zbus)?;
        let proxy = zbus::Proxy::new(connection, BUS_NAME, OBJECT_PATH, DAEMON_INTERFACE)
            .await
            .map_err(DaemonError::from_zbus)?;
        proxy
            .call(method, body)
            .await
            .map_err(DaemonError::from_zbus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_match_methods() {
        assert_eq!(route("GET", "/api/status"), Route::Status);
        assert_eq!(route("GET", "/api/incidents"), Route::Incidents);
        assert_eq!(route("GET", "/api/config"), Route::GetConfig);
        assert_eq!(route("PUT", "/api/config"), Route::PutConfig);
        assert_eq!(route("POST", "/api/chat"), Route::SendChat);
        assert_eq!(route("POST", "/api/status"), Route::MethodNotAllowed);
        assert_eq!(route("DELETE", "/api/config"), Route::MethodNotAllowed);
        assert_eq!(route("GET", "/api/unknown"), Route::NotFound);
    }

    #[test]
    fn reserved_methods_read_as_unavailable() {
        let err = DaemonError::from_error_name(
            "org.freedesktop.DBus.Error.NotSupported",
            Some("This method is reserved for a later XBot phase".into()),
        );
        assert_eq!(
            err.reply(),
            Reply::ok(json!({
                "state": "unavailable",
                "message": "This method is reserved for a later XBot phase"
            }))
        );
    }

    #[test]
    fn missing_daemon_reads_as_offline() {
        let err = DaemonError::from_error_name("org.freedesktop.DBus.Error.ServiceUnknown", None);
        assert!(matches!(err, DaemonError::Offline(_)));
        assert_eq!(err.reply().body["state"], "offline");
    }

    #[test]
    fn other_failures_are_errors() {
        let err = DaemonError::from_error_name("org.freedesktop.DBus.Error.Failed", None);
        assert_eq!(err.reply().status, 502);
    }

    fn temp_config() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xbot-chat-test-{}-{:?}/config.toml",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    #[tokio::test]
    async fn rejects_bad_requests_without_the_daemon() {
        let path = temp_config();
        let api = Api::new(path.clone());
        assert_eq!(api.handle("POST", "/api/chat", b"{").await.status, 400);
        assert_eq!(
            api.handle("POST", "/api/chat", br#"{"text":"  "}"#)
                .await
                .status,
            400
        );
        assert_eq!(api.handle("PUT", "/api/config", b"[]").await.status, 400);
        assert_eq!(
            api.handle("PUT", "/api/config", b"{\"general\":1}")
                .await
                .status,
            400
        );
        assert_eq!(api.handle("GET", "/api/nope", b"").await.status, 404);
        assert!(!path.exists(), "rejected settings must not be written");
    }

    #[tokio::test]
    async fn settings_round_trip_through_the_api() {
        let path = temp_config();
        let api = Api::new(path.clone());
        let initial = api.handle("GET", "/api/config", b"").await;
        assert_eq!(initial.status, 200);
        assert_eq!(initial.body["config"]["general"]["popups"], true);

        let mut next = initial.body["config"].clone();
        next["general"]["popups"] = json!(false);
        next["privacy"]["web_search"] = json!(true);
        let saved = api
            .handle("PUT", "/api/config", next.to_string().as_bytes())
            .await;
        assert_eq!(saved.status, 200);

        let reloaded = Config::load_from(path.clone()).unwrap();
        assert!(!reloaded.general.popups);
        assert!(reloaded.privacy.web_search);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
