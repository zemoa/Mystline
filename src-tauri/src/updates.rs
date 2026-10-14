use serde::Serialize;
use std::{io::Write, sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    sequence: u64,
    phase: &'static str,
    version: Option<String>,
    message: String,
}

struct Download {
    update: Update,
    file: tempfile::NamedTempFile,
}

pub struct Updates {
    status: Mutex<Status>,
    // Un seul téléchargement ou installateur, même avec plusieurs WebViews.
    download: tokio::sync::Mutex<Option<Download>>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            status: Mutex::new(Status {
                sequence: 0,
                phase: "disabled",
                version: None,
                message: "Mises à jour désactivées dans cette distribution.".into(),
            }),
            download: tokio::sync::Mutex::new(None),
        }
    }
}

fn supported(debug: bool, linux: bool, appimage: bool) -> bool {
    !debug && (!linux || appimage)
}

fn enabled() -> bool {
    supported(
        cfg!(debug_assertions),
        cfg!(target_os = "linux"),
        std::env::var_os("APPIMAGE").is_some(),
    )
}

fn publish(app: &AppHandle, phase: &'static str, version: Option<String>, message: String) {
    let state = app.state::<Updates>();
    let mut status = state.status.lock().expect("état des mises à jour");
    status.sequence += 1;
    status.phase = phase;
    status.version = version;
    status.message = message;
    let _ = app.emit("update-status", status.clone());
}

#[tauri::command]
pub fn update_status(app: AppHandle) -> Status {
    app.state::<Updates>()
        .status
        .lock()
        .expect("état des mises à jour")
        .clone()
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    let state = app.state::<Updates>();
    let Ok(mut pending) = state.download.try_lock() else {
        return Ok(());
    };
    if pending.is_some() {
        return Ok(());
    }
    publish(
        &app,
        "checking",
        None,
        "Recherche d’une mise à jour…".into(),
    );
    let result: Result<(), String> = async {
        let updater = app
            .updater_builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;
        let Some(mut update) = updater.check().await.map_err(|error| error.to_string())? else {
            publish(&app, "current", None, "Mystline est à jour.".into());
            return Ok(());
        };
        let version = update.version.clone();
        publish(
            &app,
            "downloading",
            Some(version.clone()),
            format!("Téléchargement de la version {version}…"),
        );
        update.timeout = Some(Duration::from_secs(300));
        // download vérifie la signature et la version signée avant de rendre les octets.
        let bytes = update
            .download(|_, _| {}, || {})
            .await
            .map_err(|error| error.to_string())?;
        let mut file = tempfile::NamedTempFile::new().map_err(|error| error.to_string())?;
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        *pending = Some(Download { update, file });
        publish(
            &app,
            "ready",
            Some(version.clone()),
            format!("La version {version} est prête à être installée."),
        );
        Ok(())
    }
    .await;
    if let Err(error) = &result {
        tracing::warn!(%error, "update_check_failed");
        publish(
            &app,
            "error",
            None,
            "Vérification impossible. Réessayez lorsque la connexion est disponible.".into(),
        );
    }
    result
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    let state = app.state::<Updates>();
    let pending = state
        .download
        .try_lock()
        .map_err(|_| "Une mise à jour est déjà en cours.")?;
    let download = pending.as_ref().ok_or("Aucune mise à jour téléchargée.")?;
    let bytes = std::fs::read(download.file.path()).map_err(|error| error.to_string())?;
    // La garde du runtime reste détenue jusqu’au redémarrage pour empêcher une écriture concurrente.
    crate::desktop::install_update(&app, &download.update, &bytes)
}

pub fn start(app: AppHandle) {
    if !enabled() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = update_check(app.clone()).await;
            tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seul_un_paquet_compatible_en_release_contacte_github() {
        assert!(!supported(true, false, false));
        assert!(!supported(true, true, true));
        assert!(!supported(false, true, false));
        assert!(supported(false, true, true));
        assert!(supported(false, false, false));
    }
}
