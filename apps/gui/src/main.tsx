import { StrictMode, useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  available,
  type Command,
  canUninstall,
  describeError,
  request,
  type Snapshot,
  setTrayLanguage,
  uninstall,
} from "./api";
import {
  batteryState,
  capabilityReason,
  isLanguageChoice,
  type LanguageChoice,
  languageNames,
  languageStorageKey,
  readLanguage,
  resolveLocale,
  translate,
} from "./i18n";
import { chargingAboveLimit } from "./policy-status";
import "./style.css";

function App() {
  const [language, setLanguage] = useState<LanguageChoice>(() => {
    try {
      return readLanguage(window.localStorage);
    } catch {
      return "system";
    }
  });
  const [systemLanguages, setSystemLanguages] = useState(
    () => navigator.languages,
  );
  const locale = resolveLocale(language, systemLanguages);
  const t = (
    key: Parameters<typeof translate>[1],
    values?: Record<string, string | number>,
  ) => translate(locale, key, values);
  useEffect(() => {
    const onLanguageChange = () => setSystemLanguages(navigator.languages);
    window.addEventListener("languagechange", onLanguageChange);
    return () => window.removeEventListener("languagechange", onLanguageChange);
  }, []);
  useEffect(() => {
    document.documentElement.lang = locale;
    void setTrayLanguage(locale).catch((error: unknown) =>
      console.error("Tray language update failed", error),
    );
  }, [locale]);
  const [uninstallAvailable, setUninstallAvailable] = useState(false);
  const [confirmUninstall, setConfirmUninstall] = useState(false);
  const [uninstallError, setUninstallError] = useState<unknown>(null);
  useEffect(() => {
    void canUninstall()
      .then(setUninstallAvailable)
      .catch(() => {});
  }, []);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [updated, setUpdated] = useState(false);
  const [busy, setBusy] = useState(false);
  const [limit, setLimit] = useState(80);
  const [start, setStart] = useState(75);
  const [useStart, setUseStart] = useState(false);
  const [report, setReport] = useState<string | null>(null);
  const generation = useRef(0);
  const edited = useRef(false);
  const mutating = useRef(false);
  const refresh = useCallback(async () => {
    if (mutating.current) return;
    const id = ++generation.current;
    try {
      const response = await request({ command: "status" });
      if (id !== generation.current) return;
      if (response.type !== "status")
        throw new Error("Unexpected status response");
      setSnapshot(response.data);
      setError(null);
      if (!edited.current) {
        setLimit(response.data.config?.charge_limit ?? 80);
        setUseStart(response.data.config?.start_threshold.enabled ?? false);
        setStart(response.data.config?.start_threshold.value ?? 75);
      }
    } catch (e) {
      if (id === generation.current) setError(e);
    }
  }, []);
  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => {
      if (!document.hidden) void refresh();
    }, 30_000);
    const onFocus = () => {
      void refresh();
    };
    window.addEventListener("focus", onFocus);
    return () => {
      ++generation.current;
      window.clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
  }, [refresh]);
  const readOnly = snapshot?.config == null;
  const act = async (command: Command) => {
    if (readOnly && command.command !== "probe") return;
    mutating.current = true;
    setBusy(true);
    setUpdated(false);
    setError(null);
    ++generation.current;
    try {
      const result = await request(command);
      if (result.type === "error") throw result.data;
      if (result.type === "probe")
        setReport(JSON.stringify(result.data, null, 2));
      else {
        edited.current = false;
        setUpdated(true);
        mutating.current = false;
        await refresh();
      }
    } catch (e) {
      setError(e);
    } finally {
      mutating.current = false;
      setBusy(false);
    }
  };
  const battery = snapshot?.batteries[0];
  const caps = snapshot?.capabilities;
  const supported = available(caps?.charge_limit);
  const entries = caps
    ? ([
        [t("chargeLimit"), caps.charge_limit],
        [t("startThreshold"), caps.start_threshold],
        [t("stopThreshold"), caps.stop_threshold],
        [t("forceDischarge"), caps.force_discharge],
        [t("adapterDisable"), caps.adapter_disable],
        [t("topUpFeature"), caps.temporary_full_charge],
      ] as const)
    : [];
  return (
    <main>
      <header>
        <div className="brand-mark" aria-hidden="true">
          <span />
        </div>
        <div>
          <strong>BatteryBridge</strong>
          <span className="eyebrow">{t("tagline")}</span>
        </div>
        <button
          type="button"
          className="refresh"
          aria-label={t("refresh")}
          disabled={busy}
          onClick={() => void refresh()}
        >
          ↻
        </button>
      </header>
      <section className="battery-view" aria-label={t("batteryStatus")}>
        <svg
          className="charge-orbit"
          viewBox="0 0 240 240"
          aria-hidden="true"
          focusable="false"
        >
          <title>Decorative orbital grid</title>
          <g className="orbit-lines" fill="none">
            <circle cx="120" cy="120" r="96" />
            <circle cx="120" cy="120" r="72" strokeDasharray="1 7" />
            <ellipse cx="120" cy="120" rx="42" ry="96" />
            <ellipse cx="120" cy="120" rx="96" ry="36" />
            <path d="M12 120h216M120 12v216" />
          </g>
          <g className="orbit-points">
            <circle cx="120" cy="24" r="3" />
            <circle cx="203" cy="72" r="4" />
            <circle cx="188" cy="188" r="3" />
            <circle cx="52" cy="52" r="3" />
            <circle cx="48" cy="120" r="2" />
            <circle cx="120" cy="192" r="2" />
          </g>
        </svg>
        <span className="eyebrow battery-label">{t("batteryStatus")}</span>
        <div className="charge">
          <span>{battery?.percent ?? "—"}</span>
          <span className="percent">%</span>
        </div>
        <progress
          className="battery-track"
          aria-label={t("batteryCharge")}
          max={100}
          value={battery?.percent ?? undefined}
        />
        <p>
          <span className="dot" />
          {(battery ? batteryState(locale, battery.state) : null) ??
            (snapshot
              ? t("noBattery")
              : error
                ? t("unavailable")
                : t("connecting"))}
          {battery?.on_ac != null && (
            <span className="power">
              {battery.on_ac ? t("ac") : t("onBattery")}
            </span>
          )}
        </p>
      </section>
      {error != null && (
        <div className="notice error" role="alert">
          <strong>{t("requestTitle")}</strong>
          <p>{describeError(error, t("requestFailed"))}</p>
          <small>{t("retry")}</small>
        </div>
      )}
      {snapshot?.last_error != null && !readOnly && (
        <div className="notice error" role="alert">
          {t("policyError")}:{" "}
          {describeError(snapshot.last_error, t("requestFailed"))}
        </div>
      )}
      {snapshot && readOnly && (
        <div className="notice" role="status">
          <strong>{t("readOnly")}</strong>
          <p>{t("readOnlyBody")}</p>
        </div>
      )}
      {caps?.charge_limit.state === "experimental" && (
        <p className="notice">{t("experimentalNotice")}</p>
      )}
      {chargingAboveLimit(snapshot) && (
        <p className="notice error" role="alert">
          {t("limitExceeded")}
        </p>
      )}
      {["Apple Silicon SMC", "Intel SMC"].includes(snapshot?.backend ?? "") && (
        <p className="notice">{t("sleepNotice")}</p>
      )}
      {updated && (
        <p className="notice" role="status">
          {t("updated")}
        </p>
      )}
      {supported ? (
        <section className="policy">
          <div className="section-heading">
            <div className="section-title">
              <span className="section-mark" aria-hidden="true">
                ✳
              </span>
              <h1>{t("chargeLimit")}</h1>
            </div>
            <span className="policy-state">
              {readOnly
                ? t("settingsUnavailable")
                : snapshot?.top_up_active
                  ? t("topUpActive")
                  : snapshot?.config?.charge_limit == null
                    ? t("off")
                    : t("activeLimit", {
                        percent: snapshot.config.charge_limit,
                      })}
            </span>
          </div>
          <p className="muted">{t("firstBattery")}</p>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void act({
                command: "limit",
                stop: limit,
                start: useStart ? start : null,
              });
            }}
          >
            <label className="limit-input" htmlFor="stop-threshold">
              {t("stopAt")}{" "}
              {caps?.allowed_limits ? (
                <select
                  id="stop-threshold"
                  value={limit}
                  disabled={busy || readOnly}
                  onChange={(e) => {
                    edited.current = true;
                    setLimit(Number(e.target.value));
                  }}
                >
                  {caps.allowed_limits.map((v) => (
                    <option key={v} value={v}>
                      {v}%
                    </option>
                  ))}
                </select>
              ) : (
                <span>
                  <input
                    id="stop-threshold"
                    aria-label={t("stopPercentage")}
                    type="number"
                    min="1"
                    max="100"
                    value={limit}
                    disabled={busy || readOnly}
                    onChange={(e) => {
                      edited.current = true;
                      setLimit(Number(e.target.value));
                    }}
                    required
                  />{" "}
                  %
                </span>
              )}
            </label>
            {available(caps?.start_threshold) && (
              <div className="start-setting">
                <label htmlFor="use-start-threshold">
                  <input
                    id="use-start-threshold"
                    type="checkbox"
                    checked={useStart}
                    disabled={busy || readOnly}
                    onChange={(e) => {
                      edited.current = true;
                      setUseStart(e.target.checked);
                    }}
                  />{" "}
                  {t("useStart")}
                </label>
                {useStart && (
                  <label htmlFor="start-threshold">
                    {t("resumeAt")}{" "}
                    <input
                      id="start-threshold"
                      aria-label={t("startPercentage")}
                      type="number"
                      min="0"
                      max={limit - 1}
                      value={start}
                      disabled={busy || readOnly}
                      required
                      onChange={(e) => {
                        edited.current = true;
                        setStart(Number(e.target.value));
                      }}
                    />{" "}
                    %
                  </label>
                )}
              </div>
            )}
            <div className="actions">
              <button
                type="submit"
                className="primary"
                disabled={busy || readOnly || !battery || !!error}
              >
                {t("apply")}
              </button>
              <button
                type="button"
                disabled={busy || readOnly || !!error}
                onClick={() =>
                  void act({ command: "limit", stop: null, start: null })
                }
              >
                {t("turnOff")}
              </button>
            </div>
          </form>
          {available(caps?.temporary_full_charge) ? (
            <button
              type="button"
              className="top-up"
              disabled={
                busy ||
                !battery ||
                !!error ||
                snapshot?.config?.charge_limit == null ||
                snapshot?.top_up_active
              }
              onClick={() => void act({ command: "top_up" })}
            >
              {t("topUp")} <span>↗</span>
            </button>
          ) : (
            <p className="muted">
              {t("topUpUnavailable")}:{" "}
              {caps &&
                capabilityReason(locale, caps.temporary_full_charge.reason)}
            </p>
          )}
        </section>
      ) : (
        snapshot && (
          <section className="notice">
            <h1>{t("limitUnavailable")}</h1>
            <p>{caps && capabilityReason(locale, caps.charge_limit.reason)}</p>
            <p>
              {t("detectedDevice")}: {snapshot.device.vendor}{" "}
              {snapshot.device.model}
            </p>
          </section>
        )
      )}
      {snapshot && (
        <>
          <section className="backend">
            <span className="eyebrow">{t("backend")}</span>
            <strong>{snapshot.backend}</strong>
            <p>
              {snapshot.device.vendor} {snapshot.device.model}
            </p>
          </section>
          <details>
            <summary>{t("capabilities")}</summary>
            <ul className="capabilities">
              {entries.map(([name, cap]) => (
                <li key={name}>
                  <div>
                    <strong>{name}</strong>
                    <span className={cap.state}>{t(cap.state)}</span>
                  </div>
                  <p>{capabilityReason(locale, cap.reason)}</p>
                </li>
              ))}
            </ul>
          </details>
          <button
            type="button"
            className="report-button"
            disabled={busy}
            onClick={() => void act({ command: "probe" })}
          >
            {t("reportButton")}
          </button>
        </>
      )}
      {report && (
        <section className="report">
          <h2>{t("reportTitle")}</h2>
          <p className="muted">{t("reportPrivacy")}</p>
          <textarea
            aria-label={t("reportJson")}
            readOnly
            value={report}
            rows={10}
          />
          <a
            download="batterybridge-probe.json"
            href={`data:application/json;charset=utf-8,${encodeURIComponent(report)}`}
          >
            {t("saveReport")}
          </a>
        </section>
      )}
      <div className="language-setting">
        <label htmlFor="language">{t("language")}</label>
        <select
          id="language"
          value={language}
          onChange={(event) => {
            const choice = event.target.value;
            if (!isLanguageChoice(choice)) return;
            setLanguage(choice);
            try {
              window.localStorage.setItem(languageStorageKey, choice);
            } catch {
              /* The choice still works for this session. */
            }
          }}
        >
          <option value="system">{t("systemLanguage")}</option>
          {Object.entries(languageNames).map(([code, name]) => (
            <option key={code} value={code} lang={code}>
              {name}
            </option>
          ))}
        </select>
      </div>
      {uninstallAvailable && (
        <section className="uninstall-setting">
          {confirmUninstall ? (
            <>
              <p>{t("uninstallConfirm")}</p>
              <div className="actions">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => {
                    mutating.current = true;
                    ++generation.current;
                    setBusy(true);
                    setUninstallError(null);
                    void uninstall().catch((e: unknown) => {
                      setUninstallError(e);
                      mutating.current = false;
                      setBusy(false);
                    });
                  }}
                >
                  {t("uninstall")}
                </button>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => setConfirmUninstall(false)}
                >
                  {t("cancel")}
                </button>
              </div>
            </>
          ) : (
            <button
              type="button"
              disabled={busy}
              onClick={() => setConfirmUninstall(true)}
            >
              {t("uninstall")}
            </button>
          )}
          {uninstallError != null && (
            <p className="notice error" role="alert">
              {describeError(uninstallError, t("requestFailed"))}
            </p>
          )}
        </section>
      )}
      <footer>{t("footer")}</footer>
    </main>
  );
}
const root = document.getElementById("root");
if (!root) throw new Error("BatteryBridge root element is missing");
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
