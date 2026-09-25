import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { process } from "@seelen-ui/lib/tauri";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { Button, Modal, Skeleton } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";

import cs from "./index.module.css";

interface NaiApp {
  id: string;
  name: string;
  description: string;
  kind: string;
  path?: string;
  capabilities: string[];
  runningPids: number[];
}

interface StoreEntry {
  id: string;
  name: string;
  description: string;
  publisher?: string;
  distributionType: string;
  sourcePackageId?: string;
  downloadUrl?: string;
  categories?: string[];
  license?: string;
  nonAutomated?: boolean;
}

interface RecentWindow {
  hwnd: number;
  title: string;
  displayTitle: string;
  application: string;
}

const PINNED_KEY = "launcher:pinned";

function loadPinned(): string[] {
  try {
    return JSON.parse(localStorage.getItem(PINNED_KEY) || "[]");
  } catch {
    return [];
  }
}

export function NaiLauncher() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const [apps, setApps] = useState<NaiApp[] | null>(null);
  const [recent, setRecent] = useState<RecentWindow[]>([]);
  const [pinned, setPinned] = useState<string[]>(loadPinned);

  async function refresh() {
    const [appList, recentWindows] = await Promise.all([
      invoke(SeelenCommand.NaiApps),
      invoke(SeelenCommand.WegGetRecentWindows, { limit: 8 }),
    ]);
    setApps(appList as unknown as NaiApp[]);
    setRecent(recentWindows as unknown as RecentWindow[]);
  }

  useEffect(() => {
    refresh();
  }, []);

  function togglePin(id: string) {
    const next = pinned.includes(id) ? pinned.filter((p) => p !== id) : [...pinned, id];
    localStorage.setItem(PINNED_KEY, JSON.stringify(next));
    setPinned(next);
  }

  async function launchApp(id: string) {
    await invoke(SeelenCommand.NaiLaunch, { id });
    refresh();
  }

  async function focusWindow(hwnd: number) {
    await invoke(SeelenCommand.WegFocusWindow, { identification: String(hwnd) });
  }

  const pinnedApps = (apps ?? []).filter((app) => pinned.includes(app.id));

  return (
    <>
      <div className={cs.hero}>
        <h1>NAI OS</h1>
        <p>{t("nai.tagline")}</p>
        <div className={cs.heroActions}>
          <Button type="primary" onClick={() => launchApp("memory")}>
            Kelvin Workstation AI
          </Button>
          <Button onClick={() => launchApp("voice")}>{t("nai.open_jarvis")}</Button>
          <Button onClick={() => navigate("/store")}>{t("header.labels.store")}</Button>
        </div>
      </div>

      <section className={cs.group}>
        <h2>{t("nai.nai_apps")}</h2>
        {!apps && <Skeleton active paragraph={{ rows: 3 }} />}
        <div className={cs.cards}>
          {(apps ?? []).map((app) => (
            <div key={app.id} className={cs.card}>
              <div className={cs.cardHeader}>
                <b>{app.name}</b>
                {app.runningPids.length > 0 && <span className={cs.badge}>●</span>}
              </div>
              <p className={cs.cardDesc}>{app.description}</p>
              <div className={cs.cardFooter}>
                <span className={cs.state}>
                  {app.path ? (app.runningPids.length ? t("nai.running") : t("nai.installed")) : t("nai.not_installed")}
                </span>
                <div>
                  <Button size="small" onClick={() => togglePin(app.id)}>
                    {pinned.includes(app.id) ? "★" : "☆"}
                  </Button>
                  <Button
                    size="small"
                    type="primary"
                    disabled={!app.path}
                    onClick={() => launchApp(app.id)}
                  >
                    {t("open")}
                  </Button>
                </div>
              </div>
            </div>
          ))}
        </div>
      </section>

      {pinnedApps.length > 0 && (
        <section className={cs.group}>
          <h2>{t("nai.pinned")}</h2>
          <div className={cs.rows}>
            {pinnedApps.map((app) => (
              <div key={app.id} className={cs.row}>
                <span>{app.name}</span>
                <Button size="small" disabled={!app.path} onClick={() => launchApp(app.id)}>
                  {t("open")}
                </Button>
              </div>
            ))}
          </div>
        </section>
      )}

      {recent.length > 0 && (
        <section className={cs.group}>
          <h2>{t("nai.recent")}</h2>
          <div className={cs.rows}>
            {recent.map((w) => (
              <div key={w.hwnd} className={cs.row}>
                <span title={w.title}>{w.displayTitle || w.title}</span>
                <Button size="small" onClick={() => focusWindow(w.hwnd)}>
                  {t("open")}
                </Button>
              </div>
            ))}
          </div>
        </section>
      )}

      <section className={cs.group}>
        <h2>{t("nai.system")}</h2>
        <div className={cs.rows}>
          <div className={cs.row}>
            <span>{t("weg.label")}</span>
            <Button size="small" onClick={() => navigate(`/widget?${new URLSearchParams({ id: "@seelen/weg" })}`)}>
              {t("open")}
            </Button>
          </div>
          <div className={cs.row}>
            <span>{t("wm.layout")}</span>
            <Button
              size="small"
              onClick={() => navigate(`/widget?${new URLSearchParams({ id: "@seelen/window-manager" })}`)}
            >
              {t("open")}
            </Button>
          </div>
          <div className={cs.row}>
            <span>{t("header.labels.store")}</span>
            <Button size="small" onClick={() => navigate("/store")}>
              {t("open")}
            </Button>
          </div>
          <div className={cs.row}>
            <span>{t("header.labels.agent")}</span>
            <Button size="small" onClick={() => navigate("/agent")}>
              {t("open")}
            </Button>
          </div>
          <div className={cs.row}>
            <span>{t("header.labels.platform")}</span>
            <Button size="small" onClick={() => navigate("/platform")}>
              {t("open")}
            </Button>
          </div>
          <div className={cs.row}>
            <span>{t("header.labels.developer")}</span>
            <Button size="small" onClick={() => navigate("/developer")}>
              {t("open")}
            </Button>
          </div>
          <div className={cs.row}>
            <span>{t("nai.restart")}</span>
            <Button
              size="small"
              onClick={() =>
                Modal.confirm({
                  title: t("action.confirm"),
                  content: t("action.confirm_body"),
                  okText: t("yes"),
                  cancelText: t("no"),
                  onOk: () => process.relaunch(),
                })}
            >
              {t("nai.restart")}
            </Button>
          </div>
        </div>
      </section>
      <Icon iconName="TbHome" />
    </>
  );
}

interface CapabilityDescriptor {
  id: string;
  risk: string;
  interactiveConfirmation: string;
  undo: string;
  latencyClass: string;
  providerPriority: string[];
}

interface ActivityItem {
  id: string;
  name: string;
  objects: string[];
}

interface CapsuleItem {
  id: string;
  name: string;
  activity: string;
  objects: string[];
  modelProfile: string;
  qosProfile: string;
}

interface Source<T> {
  ok: boolean;
  value: T | null;
  error: string | null;
}

async function source<T>(command: (typeof SeelenCommand)[keyof typeof SeelenCommand], arg?: unknown): Promise<Source<T>> {
  try {
    const value = (await invoke(command as never, arg as never)) as T;
    return { ok: true, value: value ?? null, error: null };
  } catch (error) {
    return { ok: false, value: null, error: String(error) };
  }
}

/** Agent runtime, security policy and QoS/model-residency surface (ADR/09). */
export function NaiAgent() {
  const { t } = useTranslation();
  const [caps, setCaps] = useState<Source<CapabilityDescriptor[]> | null>(null);
  const [activities, setActivities] = useState<Source<ActivityItem[]> | null>(null);
  const [capsules, setCapsules] = useState<Source<CapsuleItem[]> | null>(null);
  const [gateway, setGateway] = useState<Source<Record<string, unknown>> | null>(null);
  const [trace, setTrace] = useState<unknown[]>([]);

  async function refresh() {
    const [c, a, k, g] = await Promise.all([
      source<CapabilityDescriptor[]>(SeelenCommand.NaiCapabilities),
      source<ActivityItem[]>(SeelenCommand.NaiActivities),
      source<CapsuleItem[]>(SeelenCommand.NaiCapsules),
      source<Record<string, unknown>>(SeelenCommand.NaiGatewayModels),
    ]);
    setCaps(c);
    setActivities(a);
    setCapsules(k);
    setGateway(g);
    setTrace((await source<unknown[]>(SeelenCommand.DebugGetWidgetsStatuses).catch(() => null))?.value ?? []);
  }

  useEffect(() => {
    refresh();
  }, []);

  async function activate(id: string) {
    await invoke(SeelenCommand.NaiActivate, { identification: id });
    refresh();
  }

  async function undo() {
    await invoke(SeelenCommand.NaiUndoLast);
    refresh();
  }

  return (
    <>
      {!caps && <Skeleton active paragraph={{ rows: 4 }} />}
      <section className={cs.group}>
        <h2>
          {t("nai.capabilities")}
          <Button size="small" onClick={undo} disabled={!caps?.value?.length}>
            {t("nai.undo")}
          </Button>
        </h2>
        {caps && !caps.ok && <p className={cs.meta}>{caps.error}</p>}
        <div className={cs.rows}>
          {(caps?.value ?? []).map((cap) => (
            <div key={cap.id} className={cs.row}>
              <span>{cap.id}</span>
              <span className={cs.meta}>
                {cap.risk} · {cap.latencyClass} · {cap.undo} · {cap.providerPriority.join("/")}
              </span>
            </div>
          ))}
        </div>
      </section>

      <section className={cs.group}>
        <h2>{t("nai.activities")}</h2>
        {activities && !activities.ok && <p className={cs.meta}>{activities.error}</p>}
        <div className={cs.rows}>
          {(activities?.value ?? []).map((activity) => (
            <div key={activity.id} className={cs.row}>
              <span>{activity.name}</span>
              <div>
                <span className={cs.meta}>{activity.objects.length}</span>
                <Button size="small" onClick={() => activate(activity.id)}>
                  {t("open")}
                </Button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className={cs.group}>
        <h2>{t("nai.capsules")}</h2>
        {capsules && !capsules.ok && <p className={cs.meta}>{capsules.error}</p>}
        <div className={cs.rows}>
          {(capsules?.value ?? []).map((capsule) => (
            <div key={capsule.id} className={cs.row}>
              <span>{capsule.name}</span>
              <span className={cs.meta}>
                {capsule.activity} · {capsule.modelProfile} · {capsule.qosProfile}
              </span>
            </div>
          ))}
        </div>
      </section>

      <section className={cs.group}>
        <h2>{t("nai.gateway")}</h2>
        {gateway && !gateway.ok && <p className={cs.meta}>{gateway.error}</p>}
        {gateway?.value && (
          <pre className={cs.meta}>{JSON.stringify(gateway.value, null, 2)}</pre>
        )}
      </section>

      {trace.length > 0 && (
        <section className={cs.group}>
          <h2>{t("devtools.widgets_debug.tab")}</h2>
          <div className={cs.rows}>
            {(trace as { widgetId?: string; status?: string }[]).map((row, i) => (
              <div key={`${row.widgetId}-${i}`} className={cs.row}>
                <span>{row.widgetId}</span>
                <span className={cs.meta}>{row.status}</span>
              </div>
            ))}
          </div>
        </section>
      )}
    </>
  );
}

/** Developer platform surface: control planes + live semantic graph (ADR/10). */
export function NaiPlatform() {
  const { t } = useTranslation();
  const [gateway, setGateway] = useState<Source<Record<string, unknown>> | null>(null);
  const [graph, setGraph] = useState<Source<{ nodes: { id: string; kind: string; title: string }[] }> | null>(null);

  async function refresh() {
    const [g, n] = await Promise.all([
      source<Record<string, unknown>>(SeelenCommand.NaiGatewayModels),
      source<{ nodes: { id: string; kind: string; title: string }[] }>(SeelenCommand.NaiGraph),
    ]);
    setGateway(g);
    setGraph(n);
  }

  useEffect(() => {
    refresh();
  }, []);

  const nodes = graph?.value?.nodes ?? [];

  return (
    <>
      {!gateway && !graph && <Skeleton active paragraph={{ rows: 4 }} />}
      <section className={cs.group}>
        <h2>{t("nai.gateway")}</h2>
        {gateway && !gateway.ok && <p className={cs.meta}>{gateway.error}</p>}
        {gateway?.value && <pre className={cs.meta}>{JSON.stringify(gateway.value, null, 2)}</pre>}
      </section>

      <section className={cs.group}>
        <h2>{t("nai.control_planes")}</h2>
        <div className={cs.rows}>
          <div className={cs.row}>
            <span>MCP</span>
            <span className={cs.meta}>/mcp (JSON-RPC 2.0, tools/list + tools/call)</span>
          </div>
          <div className={cs.row}>
            <span>REST</span>
            <span className={cs.meta}>/v1/nai/*, /v1/store/*</span>
          </div>
          <div className={cs.row}>
            <span>CLI</span>
            <span className={cs.meta}>slu nai &lt;subcommand&gt;</span>
          </div>
        </div>
      </section>

      <section className={cs.group}>
        <h2>{t("nai.semantic_graph")}</h2>
        {graph && !graph.ok && <p className={cs.meta}>{graph.error}</p>}
        {nodes.length === 0 && <p className={cs.meta}>{t("nai.no_results")}</p>}
        <div className={cs.rows}>
          {nodes.map((node) => (
            <div key={node.id} className={cs.row}>
              <span title={node.title}>{node.title}</span>
              <span className={cs.meta}>{node.kind}</span>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}

export function NaiStore() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<StoreEntry[] | null>(null);
  const [statuses, setStatuses] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const catalog = (await invoke(SeelenCommand.NaiCatalog)) as unknown as {
        entries: StoreEntry[];
      };
      setEntries(catalog.entries);
      const map: Record<string, string> = {};
      for (const entry of catalog.entries) {
        const res = (await invoke(SeelenCommand.NaiAppStatus, { id: entry.id })) as unknown as {
          state: string;
        };
        map[entry.id] = res.state;
      }
      setStatuses(map);
    })();
  }, []);

  async function run(action: "install" | "update" | "uninstall" | "launch", id: string) {
    setBusy(id);
    const command = {
      install: SeelenCommand.NaiInstall,
      update: SeelenCommand.NaiUpdate,
      uninstall: SeelenCommand.NaiUninstall,
      launch: SeelenCommand.NaiStoreLaunch,
    }[action];
    await invoke(command, { id });
    const res = (await invoke(SeelenCommand.NaiAppStatus, { id })) as unknown as {
      state: string;
    };
    setStatuses((prev) => ({ ...prev, [id]: res.state }));
    setBusy(null);
  }

  return (
    <>
      {!entries && <Skeleton active paragraph={{ rows: 4 }} />}
      <div className={cs.cards}>
        {(entries ?? []).map((entry) => (
          <div key={entry.id} className={cs.card}>
            <div className={cs.cardHeader}>
              <b>{entry.name}</b>
              <span className={cs.state}>{statuses[entry.id] ?? t("nai.unknown")}</span>
            </div>
            <p className={cs.cardDesc}>{entry.description}</p>
            <p className={cs.meta}>
              {entry.publisher} · {entry.distributionType}
              {entry.sourcePackageId ? ` · ${entry.sourcePackageId}` : ""}
              {entry.license ? ` · ${entry.license}` : ""}
            </p>
            <div className={cs.cardFooter}>
              <div>
                {entry.distributionType === "web"
                  ? (
                    <Button size="small" onClick={() => run("install", entry.id)}>
                      {t("open")}
                    </Button>
                  )
                  : (
                    <>
                      <Button
                        size="small"
                        type="primary"
                        loading={busy === entry.id}
                        onClick={() => run("install", entry.id)}
                      >
                        {t("install")}
                      </Button>
                      {statuses[entry.id] === "Installed" && (
                        <>
                          <Button size="small" onClick={() => run("update", entry.id)}>
                            {t("nai.update")}
                          </Button>
                          <Button size="small" onClick={() => run("launch", entry.id)}>
                            {t("open")}
                          </Button>
                          <Button size="small" onClick={() => run("uninstall", entry.id)}>
                            {t("delete")}
                          </Button>
                        </>
                      )}
                    </>
                  )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </>
  );
}
