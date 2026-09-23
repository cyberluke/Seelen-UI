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
