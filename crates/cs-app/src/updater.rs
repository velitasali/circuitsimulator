//! Application Update Manager QObject singleton for Qt Quick.
//!
//! Runs GitHub release checks and asset downloads on background worker threads
//! and marshals state updates back onto the Qt main event loop.

use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::thread;

use cs_engine::updater::{self, BuildKind, GITHUB_RELEASES_PAGE, ReleaseAsset, ReleaseInfo};
use cs_engine::{i18n, settings};
use qtbridge::{QObjectHolder, QmlMethodInvoker, qobject};

use crate::macos_host;

enum WorkerOutcome {
    Check(Result<ReleaseInfo, String>),
    Download {
        asset_name: String,
        kind: Option<BuildKind>,
        result: Result<String, String>,
    },
}

static UPDATER_INVOKER: Mutex<Option<QmlMethodInvoker>> = Mutex::new(None);
static PENDING_OUTCOME: Mutex<Option<WorkerOutcome>> = Mutex::new(None);

extern "C" fn on_updater_worker_done() {
    if let Ok(guard) = UPDATER_INVOKER.lock()
        && let Some(inv) = guard.as_ref()
    {
        let _ = inv.invoke_method("applyWorkerResult");
    }
}

pub struct Updater {
    detected_kind: BuildKind,
    latest_release: Option<ReleaseInfo>,
    os_assets: Vec<ReleaseAsset>,
    selected_asset_index: i32,
    update_available: bool,
    checking: bool,
    downloading: bool,
    has_checked: bool,
    startup_checked: bool,
    status_text: String,
    error_text: String,
    downloaded_path: String,
    downloaded_kind: Option<BuildKind>,
    dialog_visible: bool,
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            detected_kind: updater::detect_build_kind(),
            latest_release: None,
            os_assets: Vec::new(),
            selected_asset_index: 0,
            update_available: false,
            checking: false,
            downloading: false,
            has_checked: false,
            startup_checked: false,
            status_text: String::new(),
            error_text: String::new(),
            downloaded_path: String::new(),
            downloaded_kind: None,
            dialog_visible: false,
        }
    }
}

impl Updater {
    fn ensure_invoker(&self) {
        if let Ok(mut guard) = UPDATER_INVOKER.lock()
            && guard.is_none()
        {
            *guard = Some(self.get_qml_method_invoker());
        }
    }

    fn selected_asset(&self) -> Option<&ReleaseAsset> {
        let idx = self.selected_asset_index.max(0) as usize;
        self.os_assets.get(idx)
    }

    fn start_check(&mut self) {
        if self.checking {
            return;
        }
        self.ensure_invoker();
        self.checking = true;
        self.error_text.clear();
        self.status_text = i18n::tr("Checking for updates...");
        self.state_changed();

        thread::spawn(move || {
            let res = updater::fetch_latest_release();
            if let Ok(mut guard) = PENDING_OUTCOME.lock() {
                *guard = Some(WorkerOutcome::Check(res));
            }
            macos_host::post_on_main(on_updater_worker_done);
        });
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl Updater {
    qproperty!("currentVersion", Read = current_version, Constant);
    qproperty!(
        "latestVersion",
        Read = latest_version,
        Notify = state_changed
    );
    qproperty!("releaseTitle", Read = release_title, Notify = state_changed);
    qproperty!("releaseNotes", Read = release_notes, Notify = state_changed);
    qproperty!("releaseUrl", Read = release_url, Notify = state_changed);
    qproperty!("publishedAt", Read = published_at, Notify = state_changed);
    qproperty!(
        "detectedBuildLabel",
        Read = detected_build_label,
        Notify = state_changed
    );
    qproperty!(
        "updateAvailable",
        Read = update_available,
        Notify = state_changed
    );
    qproperty!("checking", Read = checking, Notify = state_changed);
    qproperty!("downloading", Read = downloading, Notify = state_changed);
    qproperty!("hasChecked", Read = has_checked, Notify = state_changed);
    qproperty!("statusText", Read = status_text, Notify = state_changed);
    qproperty!("errorText", Read = error_text, Notify = state_changed);
    qproperty!("assets", Read = assets, Notify = state_changed);
    qproperty!(
        "selectedAssetIndex",
        Read = selected_asset_index,
        Write = set_selected_asset_index,
        Notify = state_changed
    );
    qproperty!(
        "selectedAssetName",
        Read = selected_asset_name,
        Notify = state_changed
    );
    qproperty!(
        "downloadedPath",
        Read = downloaded_path,
        Notify = state_changed
    );
    qproperty!(
        "canLaunchInstaller",
        Read = can_launch_installer,
        Notify = state_changed
    );
    qproperty!(
        "dialogVisible",
        Read = dialog_visible,
        Write = set_dialog_visible,
        Notify = dialog_visible_changed
    );

    #[qsignal]
    fn state_changed(&mut self);
    #[qsignal]
    fn dialog_visible_changed(&mut self);
    #[qsignal]
    fn request_show_dialog(&mut self);

    fn current_version(&self) -> String {
        updater::current_version()
    }

    fn latest_version(&self) -> String {
        self.latest_release
            .as_ref()
            .map(|r| r.version.clone())
            .unwrap_or_else(updater::current_version)
    }

    fn release_title(&self) -> String {
        self.latest_release
            .as_ref()
            .map(|r| r.name.clone())
            .unwrap_or_default()
    }

    fn release_notes(&self) -> String {
        self.latest_release
            .as_ref()
            .map(|r| r.body.clone())
            .unwrap_or_default()
    }

    fn release_url(&self) -> String {
        self.latest_release
            .as_ref()
            .map(|r| r.html_url.clone())
            .unwrap_or_else(|| GITHUB_RELEASES_PAGE.to_string())
    }

    fn published_at(&self) -> String {
        self.latest_release
            .as_ref()
            .map(|r| r.published_at.clone())
            .unwrap_or_default()
    }

    fn detected_build_label(&self) -> String {
        self.detected_kind.display_label()
    }

    fn update_available(&self) -> bool {
        self.update_available
    }

    fn checking(&self) -> bool {
        self.checking
    }

    fn downloading(&self) -> bool {
        self.downloading
    }

    fn has_checked(&self) -> bool {
        self.has_checked
    }

    fn status_text(&self) -> String {
        self.status_text.clone()
    }

    fn error_text(&self) -> String {
        self.error_text.clone()
    }

    fn assets(&self) -> Vec<String> {
        self.os_assets
            .iter()
            .map(ReleaseAsset::display_label)
            .collect()
    }

    fn selected_asset_index(&self) -> i32 {
        self.selected_asset_index
    }

    fn set_selected_asset_index(&mut self, index: i32) {
        let clamped = if self.os_assets.is_empty() {
            0
        } else {
            index.clamp(0, (self.os_assets.len() - 1) as i32)
        };
        if self.selected_asset_index != clamped {
            self.selected_asset_index = clamped;
            self.downloaded_path.clear();
            self.downloaded_kind = None;
            self.state_changed();
        }
    }

    fn selected_asset_name(&self) -> String {
        self.selected_asset()
            .map(|a| a.name.clone())
            .unwrap_or_default()
    }

    fn downloaded_path(&self) -> String {
        self.downloaded_path.clone()
    }

    fn can_launch_installer(&self) -> bool {
        !self.downloaded_path.is_empty()
            && self
                .downloaded_kind
                .is_some_and(BuildKind::is_direct_installer)
    }

    fn dialog_visible(&self) -> bool {
        self.dialog_visible
    }

    fn set_dialog_visible(&mut self, visible: bool) {
        if visible {
            if !self.dialog_visible {
                self.dialog_visible = true;
                self.dialog_visible_changed();
            }
            self.request_show_dialog();
        } else if self.dialog_visible {
            self.dialog_visible = false;
            self.dialog_visible_changed();
        }
    }

    /// Runs a silent background update check on startup if `auto_update` is enabled.
    #[qslot]
    fn check_on_startup(&mut self) {
        if self.startup_checked {
            return;
        }
        self.startup_checked = true;
        if !settings::get().auto_update {
            return;
        }
        self.start_check();
    }

    /// Opens the update dialog and initiates an update check against GitHub Releases.
    #[qslot]
    fn check_for_updates(&mut self) {
        self.set_dialog_visible(true);
        self.start_check();
    }

    /// Opens the update dialog; starts a check only if we have not checked yet.
    #[qslot]
    fn open_update_dialog(&mut self) {
        self.set_dialog_visible(true);
        if !self.has_checked && !self.checking {
            self.start_check();
        }
    }

    /// Downloads the currently selected release asset to the user's Downloads folder.
    #[qslot]
    fn download_selected_asset(&mut self) {
        if self.downloading || self.checking {
            return;
        }
        let Some(asset) = self.selected_asset().cloned() else {
            return;
        };

        self.ensure_invoker();
        self.downloading = true;
        self.error_text.clear();
        self.downloaded_path.clear();
        self.downloaded_kind = None;
        self.status_text = i18n::tr("Downloading %1...").replace("%1", &asset.name);
        self.state_changed();

        let dest_dir = updater::default_download_dir();
        thread::spawn(move || {
            let res = updater::download_release_asset(&asset.download_url, &asset.name, &dest_dir)
                .map(|p| p.to_string_lossy().into_owned());
            if let Ok(mut guard) = PENDING_OUTCOME.lock() {
                *guard = Some(WorkerOutcome::Download {
                    asset_name: asset.name,
                    kind: asset.kind,
                    result: res,
                });
            }
            macos_host::post_on_main(on_updater_worker_done);
        });
    }

    /// Launches or opens the downloaded update package.
    #[qslot]
    fn open_downloaded_file(&mut self) {
        if self.downloaded_path.is_empty() {
            return;
        }
        let path = Path::new(&self.downloaded_path);
        if !path.exists() {
            return;
        }
        if self.can_launch_installer() {
            if Command::new(path).spawn().is_err() {
                let _ = macos_host::open_local_path(path);
            }
        } else {
            let _ = macos_host::open_local_path(path);
        }
    }

    /// Opens the folder containing the downloaded update file.
    #[qslot]
    fn open_download_folder(&mut self) {
        if self.downloaded_path.is_empty() {
            return;
        }
        let path = Path::new(&self.downloaded_path);
        let folder = path.parent().unwrap_or(path);
        let _ = macos_host::open_local_path(folder);
    }

    /// Internal callback invoked on the Qt main thread when a background check or download finishes.
    #[qslot]
    fn apply_worker_result(&mut self) {
        let outcome = PENDING_OUTCOME.lock().ok().and_then(|mut g| g.take());
        let Some(outcome) = outcome else {
            return;
        };

        match outcome {
            WorkerOutcome::Check(Ok(info)) => {
                self.checking = false;
                self.has_checked = true;
                self.error_text.clear();

                let curr = updater::current_version();
                let is_newer = updater::is_newer_version(&curr, &info.version);
                self.update_available = is_newer;
                self.os_assets = info.assets_for_os(env::consts::OS, self.detected_kind);
                self.selected_asset_index = 0;
                self.downloaded_path.clear();
                self.downloaded_kind = None;

                if is_newer {
                    self.status_text =
                        i18n::tr("A new version (%1) is available!").replace("%1", &info.version);
                } else {
                    self.status_text =
                        i18n::tr("Circuit Simulator is up to date (%1).").replace("%1", &curr);
                }
                self.latest_release = Some(info);
                self.state_changed();
            }
            WorkerOutcome::Check(Err(err)) => {
                self.checking = false;
                self.has_checked = true;
                self.error_text = err.clone();
                let prefix = i18n::tr("Error checking updates:");
                self.status_text = format!("{prefix} {err}");
                self.state_changed();
            }
            WorkerOutcome::Download {
                asset_name,
                kind,
                result: Ok(path),
            } => {
                self.downloading = false;
                self.error_text.clear();
                self.downloaded_path = path.clone();
                self.downloaded_kind = kind;
                self.status_text = i18n::tr("Downloaded %1.").replace("%1", &asset_name);
                self.state_changed();
            }
            WorkerOutcome::Download {
                asset_name: _,
                kind: _,
                result: Err(err),
            } => {
                self.downloading = false;
                self.error_text = err.clone();
                let prefix = i18n::tr("Download failed:");
                self.status_text = format!("{prefix} {err}");
                self.state_changed();
            }
        }
    }
}
