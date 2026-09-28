use xbot_core::{Config, Status, BUS_NAME, OBJECT_PATH};

struct Daemon {
    config: Config,
}

fn future_phase<T>() -> zbus::fdo::Result<T> {
    Err(zbus::fdo::Error::NotSupported(
        "This method is reserved for a later XBot phase".into(),
    ))
}

#[zbus::interface(name = "org.cruxos.XBot1.Daemon")]
impl Daemon {
    fn status(&self) -> zbus::fdo::Result<String> {
        serde_json::to_string(&Status::from_config(&self.config))
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))
    }

    fn list_incidents(&self, _filter_json: &str) -> zbus::fdo::Result<String> {
        future_phase()
    }

    fn get_incident(&self, _id: &str) -> zbus::fdo::Result<String> {
        future_phase()
    }

    fn ignore_kind(&self, _fingerprint: &str) -> zbus::fdo::Result<()> {
        future_phase()
    }

    fn dismiss_incident(&self, _id: &str) -> zbus::fdo::Result<()> {
        future_phase()
    }

    fn start_session(&self, _origin_json: &str) -> zbus::fdo::Result<String> {
        future_phase()
    }

    fn send_message(&self, _session_id: &str, _message_json: &str) -> zbus::fdo::Result<()> {
        future_phase()
    }

    fn cancel_session(&self, _session_id: &str) -> zbus::fdo::Result<()> {
        future_phase()
    }

    fn resolve_approval(&self, _approval_id: &str, _decision_json: &str) -> zbus::fdo::Result<()> {
        future_phase()
    }

    fn undo(&self, _action_or_session_id: &str, _dry_run: bool) -> zbus::fdo::Result<String> {
        future_phase()
    }

    fn preview_outbound(&self, _session_id: &str) -> zbus::fdo::Result<String> {
        future_phase()
    }

    fn set_voice(&self, _enabled: bool) -> zbus::fdo::Result<()> {
        future_phase()
    }

    #[zbus(signal)]
    async fn session_event(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        session_id: &str,
        event_json: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn approval_requested(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        approval_json: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn incident_opened(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        incident_json: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn incident_updated(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        incident_json: &str,
    ) -> zbus::Result<()>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let _connection = zbus::connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, Daemon { config })?
        .build()
        .await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}
