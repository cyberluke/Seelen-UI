use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
    time::Duration,
};

use seelen_core::state::{Widget, WidgetInstanceMode, WidgetPreset, WidgetStatus};
use tauri::{Emitter, Listener};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use uuid::Uuid;

use crate::{
    app::get_app_handle,
    boot,
    error::ResultLogExt,
    get_tokio_handle,
    modules::monitors::MonitorManager,
    resources::RESOURCES,
    state::application::FULL_STATE,
    utils::lock_free::SyncHashMap,
    widgets::{
        WidgetWebviewLabel,
        manager::WIDGET_MANAGER,
        notify_widget_statuses_change,
        webview::{self, WidgetWebview},
    },
    windows_api::event_window::IS_INTERACTIVE_SESSION,
};

pub enum PodSource {
    Static,
    Runtime,
}

const LIVENESS_PROVE_INTERVAL: Duration = Duration::from_secs(5);
const LIVENESS_PROVE_WAIT_TIMEOUT: Duration = Duration::from_secs(3);
const LIVENESS_PROVE_MAX_RETRIES: u8 = 5;
// Grace period after session resume or soft_restart to let the webview finish reloading.
const LIVENESS_RELOAD_GRACE_PERIOD: Duration = Duration::from_secs(10);
// Widgets sit occluded/background almost all the time, so the OS never signals real
// memory pressure to their renderer and the JS heap balloons unchecked. Simulating it
// periodically forces WebView2 to actually release freed pages back to the OS.
const MEMORY_PRESSURE_INTERVAL: Duration = Duration::from_secs(30);

pub struct WidgetDeployment {
    pub definition: Arc<Widget>,
    pub pods: SyncHashMap<WidgetWebviewLabel, WidgetPod>,
}

impl WidgetDeployment {
    pub fn new(definition: Arc<Widget>) -> Self {
        log::trace!("Registering widget: {}", definition.id);
        Self {
            definition,
            pods: SyncHashMap::new(),
        }
    }

    /// Will revaluate all widget instances and remove or add them based on current user settings
    pub fn reconcile(&self) {
        match self.definition.instances {
            WidgetInstanceMode::Single => {
                if self.pods.is_empty() {
                    let label = WidgetWebviewLabel::new(&self.definition.id, None, None);
                    let instance = WidgetPod::create(label, None, PodSource::Static);
                    self.pods.upsert(instance.label.clone(), instance);
                }
            }
            WidgetInstanceMode::Multiple => {
                let nil_id = Uuid::nil();
                if !self.definition.lazy && self.pods.is_empty() {
                    let label = WidgetWebviewLabel::new(&self.definition.id, None, Some(&nil_id));
                    let instance = WidgetPod::create(label, None, PodSource::Static);
                    self.pods.upsert(instance.label.clone(), instance);
                }

                let replicas_ids = FULL_STATE
                    .load()
                    .get_widget_instances_ids(&self.definition.id);

                // Remove deleted static instances; runtime pods are never evicted by reconcile.
                self.pods.retain(|(label, pod)| {
                    if matches!(pod.source, PodSource::Runtime) {
                        return true;
                    }
                    let instance_id = label.instance_id.expect("Missing instance id");
                    instance_id == nil_id || replicas_ids.contains(&instance_id)
                });

                // Add new instances
                for replica_id in replicas_ids {
                    if !self
                        .pods
                        .any(|(label, _)| label.instance_id == Some(replica_id))
                    {
                        let label =
                            WidgetWebviewLabel::new(&self.definition.id, None, Some(&replica_id));
                        let instance = WidgetPod::create(label, None, PodSource::Static);
                        self.pods.upsert(instance.label.clone(), instance);
                    }
                }
            }
            WidgetInstanceMode::ReplicaByMonitor => {
                let configs = FULL_STATE.load();
                let connected_ids = MonitorManager::instance().get_cached_ids();

                // Remove disabled or disconnected instances
                self.pods.retain(|(label, _)| {
                    let monitor_id = label.monitor_id.as_ref().expect("Missing monitor id");
                    connected_ids.contains(monitor_id)
                        && configs.is_widget_enable_on_monitor(&self.definition.id, monitor_id)
                });

                // Add new/enabled instances
                for monitor_id in connected_ids {
                    if self
                        .pods
                        .any(|(label, _)| label.monitor_id.as_ref() == Some(&monitor_id))
                    {
                        continue;
                    }

                    if !configs.is_widget_enable_on_monitor(&self.definition.id, &monitor_id) {
                        continue;
                    }

                    let label =
                        WidgetWebviewLabel::new(&self.definition.id, Some(&monitor_id), None);
                    let instance = WidgetPod::create(label, None, PodSource::Static);
                    self.pods.upsert(instance.label.clone(), instance);
                }
            }
        }
    }

    pub fn start_all_webviews(&self) {
        self.pods.for_each(|(_k, pod)| {
            pod.run(&self.definition);
        });
    }

    pub fn start_webview(&self, label: &WidgetWebviewLabel) {
        self.pods.get(label, |pod| {
            pod.run(&self.definition);
        });
    }

    pub fn create_runtime_instance(&self, instance_id: &Uuid, owner_hwnd: Option<isize>) {
        let label = WidgetWebviewLabel::new(&self.definition.id, None, Some(instance_id));
        let instance = WidgetPod::create(label, owner_hwnd, PodSource::Runtime);
        self.pods.upsert(instance.label.clone(), instance);
    }

    pub fn kill_pod(&self, label: &WidgetWebviewLabel) {
        self.pods.remove(label);
    }
}

/// Generation + nonce ping/pong payloads. Anonymous pings are not accepted:
/// a pong only counts when both the renderer generation and the nonce match
/// the outstanding request, so stale answers from a previous generation can
/// never mask (or fake) liveness of the current one.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LivenessPing {
    generation: u64,
    nonce: u64,
    /// monotonic microseconds from process t0
    sent_at: u64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LivenessPong {
    generation: u64,
    nonce: u64,
}

pub struct WidgetPod {
    pub label: WidgetWebviewLabel,
    pub source: PodSource,

    window: Option<WidgetWebview>,
    _status: WidgetStatus,
    owner_hwnd: Option<isize>,
    preset: WidgetPreset,

    live: Arc<tokio::sync::Notify>,
    /// renderer generation: incremented on every soft restart
    generation: Arc<AtomicU64>,
    /// the ping currently awaiting a pong: (nonce, sent instant)
    outstanding: Arc<std::sync::Mutex<Option<(u64, std::time::Instant)>>>,
    liveness_prove_handle: Option<tokio::task::JoinHandle<()>>,
    memory_pressure_handle: Option<tokio::task::JoinHandle<()>>,
    retries: Arc<AtomicU8>,
}

impl WidgetPod {
    fn create(label: WidgetWebviewLabel, owner_hwnd: Option<isize>, source: PodSource) -> Self {
        let preset = RESOURCES
            .widgets
            .read_sync(&label.widget_id, |_, w| w.preset)
            .unwrap_or(WidgetPreset::None);
        Self {
            label,
            source,
            window: None,
            _status: WidgetStatus::Pending,
            owner_hwnd,
            preset,
            live: Arc::new(tokio::sync::Notify::new()),
            generation: Arc::new(AtomicU64::new(1)),
            outstanding: Arc::new(std::sync::Mutex::new(None)),
            liveness_prove_handle: None,
            memory_pressure_handle: None,
            retries: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn status(&self) -> &WidgetStatus {
        &self._status
    }

    pub fn hwnd(&self) -> Option<isize> {
        self.window.as_ref()?.0.hwnd().ok().map(|h| h.0 as isize)
    }

    pub fn set_status(&mut self, status: WidgetStatus) {
        log::trace!(target: &self.label.decoded, "status changed to: {status:?}");
        self._status = status;
        notify_widget_statuses_change();
    }

    pub fn is_ready(&self) -> bool {
        self.window.is_some() && self.status() == &WidgetStatus::Ready
    }

    pub fn soft_restart(&mut self) {
        if let Some((handle, is_popup)) = self.prepare_restart() {
            // Guards are already released at this point: perform native ops.
            if is_popup {
                let _ = handle.hide();
            }
            handle.reload().log_error();
            self.set_status(WidgetStatus::Mounting);
            boot::record_pod(&self.label.raw, "widget.mounting");
        }
    }

    /// Mutation part of a soft restart (runs under the map guard): stop old
    /// state by incrementing the renderer generation, mark `Restarting`, and
    /// snapshot the cheap handles so the caller can perform the native
    /// reload/hidden ops AFTER the guards are dropped.
    fn prepare_restart(&mut self) -> Option<(tauri::WebviewWindow, bool)> {
        // Pod was never started: leave it in Pending so run() can initialize it.
        self.window.as_ref()?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        boot::bump_generation(&self.label.raw);
        boot::record_reload(&self.label.raw);
        self.set_status(WidgetStatus::Restarting);
        log::trace!(
            "renderer generation {generation} started for {}",
            self.label.decoded
        );
        let handle = self.window.as_ref().map(|w| w.handle());
        let is_popup = matches!(self.preset, WidgetPreset::Popup);
        handle.map(|handle| (handle, is_popup))
    }

    fn run(&mut self, definition: &Widget) {
        if self.status() != &WidgetStatus::Pending {
            return;
        }

        self.set_status(WidgetStatus::Creating);
        let window = match WidgetWebview::create(definition, &self.label, self.owner_hwnd) {
            Ok(window) => window,
            Err(err) => {
                log::error!("Failed to create webview: {}", err);
                self.set_status(WidgetStatus::CrashedOnCreation);
                return;
            }
        };
        self.set_status(WidgetStatus::Mounting);

        let label = self.label.clone();
        window.0.on_window_event(move |event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Defer window creation off the UI message-loop thread.
                // Calling start_all_webviews() (→ WidgetWebview::create → builder.build)
                // synchronously here triggers a re-entrant ZwUserDestroyWindow while the
                // message pump is still inside the destruction handler, which causes the
                // APPLICATION_HANG_ENDTASK_HungThreadIsIdle crash.
                let label = label.clone();
                std::thread::spawn(move || {
                    WIDGET_MANAGER.deployments.get(&label.widget_id, |deploy| {
                        deploy.kill_pod(&label);
                        deploy.reconcile();
                        if !deploy.definition.lazy {
                            deploy.start_all_webviews();
                        }
                    });
                });
            }
        });

        if definition.debug {
            window.0.open_devtools();
        }

        self.window = Some(window);
        boot::record_pod(&self.label.raw, "widget.native_window.created");
        notify_widget_statuses_change();
        boot::record_pod(&self.label.raw, "widget.mounting");
        self.start_liveness_prove();
        self.start_memory_pressure_simulation();
    }

    fn start_memory_pressure_simulation(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let webview = window.handle();

        let handle = get_tokio_handle().spawn(async move {
            loop {
                tokio::time::sleep(MEMORY_PRESSURE_INTERVAL).await;
                webview::simulate_memory_pressure(&webview);
            }
        });

        self.memory_pressure_handle = Some(handle);
    }

    fn start_liveness_prove(&mut self) {
        let generation = self.generation.clone();
        let outstanding = self.outstanding.clone();
        let live = self.live.clone();

        if let Some(window) = &self.window {
            let label_raw = self.label.raw.clone();
            window.0.listen("internal::liveness-pong", move |event| {
                let Ok(pong) = serde_json::from_str::<LivenessPong>(event.payload()) else {
                    return;
                };
                let current = generation.load(Ordering::SeqCst);
                if pong.generation != current {
                    return; // stale pong from an older renderer generation
                }
                let mut pending = outstanding.lock().unwrap_or_else(|e| e.into_inner());
                if let Some((nonce, sent)) = *pending
                    && pong.nonce == nonce
                {
                    let rtt_us = sent.elapsed().as_micros() as u64;
                    *pending = None;
                    drop(pending);
                    boot::record_rtt(&label_raw, rtt_us);
                    live.notify_waiters();
                }
            });
        }

        let live = self.live.clone();
        let generation = self.generation.clone();
        let outstanding = self.outstanding.clone();
        let label = self.label.clone();
        let retries = self.retries.clone();

        let handle = get_tokio_handle().spawn(async move {
            let app = get_app_handle();
            let mut was_suspended = false;
            // Two phases, so failures are not conflated:
            // `!mounted` -> frontend boot never reached `Ready`;
            // `mounted`  -> widget was ready once, then stopped responding.
            // A soft restart puts the pod back into the mount phase.
            let mut mounted = false;
            let mut nonce: u64 = 0;

            loop {
                tokio::time::sleep(LIVENESS_PROVE_INTERVAL).await;
                if !IS_INTERACTIVE_SESSION.load(std::sync::atomic::Ordering::Acquire) {
                    was_suspended = true;
                    continue;
                }

                // After session resume, reset state and wait for webview to finish reloading.
                if was_suspended {
                    was_suspended = false;
                    retries.store(0, Ordering::SeqCst);
                    tokio::time::sleep(LIVENESS_RELOAD_GRACE_PERIOD).await;
                    continue;
                }

                // Mount watchdog: until the widget reaches `Ready` we only poll its status,
                // because the ping/pong handshake can't get an answer before the frontend
                // entrypoint registered the `liveness-pong` listener.
                if !mounted {
                    let is_ready = WIDGET_MANAGER
                        .deployments
                        .get(&label.widget_id, |deployment| {
                            deployment.pods.get(&label, |pod| pod.is_ready())
                        })
                        .flatten()
                        == Some(true);

                    if is_ready {
                        mounted = true;
                        retries.store(0, Ordering::SeqCst);
                        continue;
                    }

                    let attempt = retries.fetch_add(1, Ordering::SeqCst);
                    log::warn!(
                        "Mount prove failed for {label}: frontend never reached Ready (attempt {}/{LIVENESS_PROVE_MAX_RETRIES}), reloading webview.",
                        attempt + 1
                    );

                    if attempt < LIVENESS_PROVE_MAX_RETRIES {
                        boot::record_mount_failure(&label.raw);
                        Self::perform_restart(&label);
                        continue;
                    }

                    log::error!("Mount prove failed for {label} too many times, giving up.");
                    WIDGET_MANAGER.deployments.get(&label.widget_id, |deployment| {
                        deployment.pods.get(&label, |pod| {
                            pod.set_status(WidgetStatus::MountFailed);
                        });
                    });
                    boot::record_gave_up(&label.raw);
                    Self::report_dead_widget(&label).await;
                    break;
                }

                // Runtime liveness watchdog. Ping carries {generation, nonce};
                // the pong is matched by both before counting as a success.
                nonce = nonce.wrapping_add(1);
                let sent_at = crate::boot::mono_us();
                {
                    let mut pending = outstanding.lock().unwrap_or_else(|e| e.into_inner());
                    *pending = Some((nonce, std::time::Instant::now()));
                    let _ = sent_at;
                }

                // `enable` registers the waiter before the ping is emitted, so a
                // pong that arrives before the `select!` polls the future is stored
                // as a permit instead of being dropped by `notify_waiters`.
                let mut waiter = std::pin::pin!(live.notified());
                waiter.as_mut().enable();

                let ping = LivenessPing {
                    generation: generation.load(Ordering::SeqCst),
                    nonce,
                    sent_at,
                };
                let _ = app.emit_to(&label.raw, "internal::liveness-ping", &ping);

                tokio::select! {
                    _ = waiter.as_mut() => {
                        // Widget is healthy: reset consecutive failure counter.
                        retries.store(0, Ordering::SeqCst);
                    }
                    _ = tokio::time::sleep(LIVENESS_PROVE_WAIT_TIMEOUT) => {
                        {
                            let mut pending = outstanding.lock().unwrap_or_else(|e| e.into_inner());
                            *pending = None;
                        }
                        let attempt = retries.fetch_add(1, Ordering::SeqCst);
                        log::warn!("Liveness prove failed for {label}: widget stopped responding (attempt {}/{LIVENESS_PROVE_MAX_RETRIES}), reloading webview.", attempt + 1);

                        if attempt < LIVENESS_PROVE_MAX_RETRIES {
                            boot::record_liveness_failure(&label.raw);
                            Self::perform_restart(&label);
                            // back into the mount phase: wait for the new generation
                            // to reach `Ready` before resuming ping/pong.
                            mounted = false;
                        } else {
                            log::error!("Liveness prove failed for {label} too many times, giving up.");
                            WIDGET_MANAGER.deployments.get(&label.widget_id, |deployment| {
                                deployment.pods.get(&label, |pod| {
                                    pod.set_status(WidgetStatus::MountFailed);
                                });
                            });
                            boot::record_gave_up(&label.raw);
                            Self::report_dead_widget(&label).await;
                            break;
                        }
                    }
                }
            }
        });

        self.liveness_prove_handle = Some(handle);
    }

    /// Restart in the snapshot pattern: mutate under the map guards, drop the
    /// guards, then perform the native webview operations (hide / reload).
    fn perform_restart(label: &WidgetWebviewLabel) {
        let snapshot: Option<(tauri::WebviewWindow, bool)> = WIDGET_MANAGER
            .deployments
            .get(&label.widget_id, |deployment| {
                deployment
                    .pods
                    .get(label, |pod| pod.prepare_restart())
                    .flatten()
            })
            .flatten();

        if let Some((handle, is_popup)) = snapshot {
            if is_popup {
                let _ = handle.hide();
            }
            handle.reload().log_error();
            WIDGET_MANAGER
                .deployments
                .get(&label.widget_id, |deployment| {
                    deployment.pods.get(label, |pod| {
                        pod.set_status(WidgetStatus::Mounting);
                        boot::record_pod(&pod.label.raw, "widget.mounting");
                    });
                });
        }
    }

    /// Error dialog for widgets whose watchdog gave up.
    async fn report_dead_widget(label: &WidgetWebviewLabel) {
        let lang = rust_i18n::locale();
        let widget_name = RESOURCES
            .widgets
            .read_async(&label.widget_id, |_, w| {
                w.metadata.display_name.get(&lang).to_string()
            })
            .await
            .unwrap_or_else(|| label.widget_id.to_string());
        get_app_handle()
            .dialog()
            .message(t!(
                "widget_liveness.failed_description",
                widget_name = widget_name
            ))
            .title(t!("widget_liveness.failed_title"))
            .kind(MessageDialogKind::Error)
            .buttons(MessageDialogButtons::Ok)
            .show(|_| {});
    }
}

impl Drop for WidgetPod {
    fn drop(&mut self) {
        log::trace!(target: &self.label.decoded, "dropped");
        if let Some(handle) = self.liveness_prove_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.memory_pressure_handle.take() {
            handle.abort();
        }
    }
}
