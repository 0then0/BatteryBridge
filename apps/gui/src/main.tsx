import { StrictMode, useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  available,
  type Command,
  describeError,
  request,
  type Snapshot,
} from "./api";
import "./style.css";

function App() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
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
      if (id === generation.current) setError(describeError(e));
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
    setMessage(null);
    setError(null);
    ++generation.current;
    try {
      const result = await request(command);
      if (result.type === "error") throw result.data;
      if (result.type === "probe")
        setReport(JSON.stringify(result.data, null, 2));
      else {
        edited.current = false;
        setMessage("Charge policy applied.");
        mutating.current = false;
        await refresh();
      }
    } catch (e) {
      setError(describeError(e));
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
        ["Charge limit", caps.charge_limit],
        ["Start threshold", caps.start_threshold],
        ["Stop threshold", caps.stop_threshold],
        ["Force discharge", caps.force_discharge],
        ["Adapter disable", caps.adapter_disable],
        ["Top up", caps.temporary_full_charge],
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
          <span className="eyebrow">CHARGE CONTROL</span>
        </div>
        <button
          type="button"
          className="refresh"
          aria-label="Refresh battery status"
          disabled={busy}
          onClick={() => void refresh()}
        >
          ↻
        </button>
      </header>
      <section className="battery-view" aria-label="Battery status">
        <div className="charge">
          <span>{battery?.percent ?? "—"}</span>
          <span className="percent">%</span>
        </div>
        <div
          className="battery-track"
          role="progressbar"
          aria-label="Battery charge"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={battery?.percent ?? undefined}
        >
          <span style={{ width: `${battery?.percent ?? 0}%` }} />
        </div>
        <p>
          <span className="dot" />
          {battery?.state ??
            (snapshot
              ? "No battery detected"
              : error
                ? "Status unavailable"
                : "Connecting to service")}
          {battery?.on_ac != null && (
            <span className="power">
              {battery.on_ac ? "AC connected" : "On battery"}
            </span>
          )}
        </p>
      </section>
      {error && (
        <div className="notice error" role="alert">
          <strong>Service unavailable or request failed</strong>
          <p>{error}</p>
          <small>
            Start the installed BatteryBridge service, then refresh.
          </small>
        </div>
      )}
      {snapshot?.last_error != null && !readOnly && (
        <div className="notice error" role="alert">
          Policy could not be enforced: {describeError(snapshot.last_error)}
        </div>
      )}
      {snapshot && readOnly && (
        <div className="notice" role="status">
          <strong>Read-only mode</strong>
          <p>
            Battery status and compatibility reports are available. Start the
            BatteryBridge service to change charging settings.
          </p>
        </div>
      )}
      {caps?.charge_limit.state === "experimental" && (
        <p className="notice">
          Experimental hardware interface. Writes are not hardware-verified for
          this device; firmware may reject them.
        </p>
      )}
      {message && (
        <p className="notice" role="status">
          {message}
        </p>
      )}
      {supported ? (
        <section className="policy">
          <div className="section-heading">
            <h1>Charge limit</h1>
            <span>
              {readOnly
                ? "Policy unavailable"
                : snapshot?.top_up_active
                  ? "Top-up active"
                  : snapshot?.config?.charge_limit == null
                    ? "Off"
                    : `${snapshot.config?.charge_limit}% active`}
            </span>
          </div>
          <p className="muted">
            Applies to the first battery. Other packs keep their own settings.
          </p>
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
              Stop charging at{" "}
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
                    aria-label="Stop charging percentage"
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
                  Set start threshold
                </label>
                {useStart && (
                  <label htmlFor="start-threshold">
                    Resume at{" "}
                    <input
                      id="start-threshold"
                      aria-label="Start charging percentage"
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
                Apply limit
              </button>
              <button
                type="button"
                disabled={busy || readOnly || !!error}
                onClick={() =>
                  void act({ command: "limit", stop: null, start: null })
                }
              >
                Turn off
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
              Top up to 100% <span>↗</span>
            </button>
          ) : (
            <p className="muted">
              Top-up unavailable: {caps?.temporary_full_charge.reason}
            </p>
          )}
        </section>
      ) : (
        snapshot && (
          <section className="notice">
            <h1>Charge limit unavailable</h1>
            <p>{caps?.charge_limit.reason}</p>
            <p>
              Detected device: {snapshot.device.vendor} {snapshot.device.model}
            </p>
          </section>
        )
      )}
      {snapshot && (
        <>
          <section className="backend">
            <span className="eyebrow">ACTIVE BACKEND</span>
            <strong>{snapshot.backend}</strong>
            <p>
              {snapshot.device.vendor} {snapshot.device.model}
            </p>
          </section>
          <details>
            <summary>Device capabilities</summary>
            <ul className="capabilities">
              {entries.map(([name, cap]) => (
                <li key={name}>
                  <div>
                    <strong>{name}</strong>
                    <span className={cap.state}>{cap.state}</span>
                  </div>
                  <p>{cap.reason}</p>
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
            Generate compatibility report
          </button>
        </>
      )}
      {report && (
        <section className="report">
          <h2>Compatibility report</h2>
          <p className="muted">
            Only allowlisted fields are collected. Review the report before
            sharing.
          </p>
          <textarea
            aria-label="Compatibility report JSON"
            readOnly
            value={report}
            rows={10}
          />
          <a
            download="batterybridge-probe.json"
            href={`data:application/json;charset=utf-8,${encodeURIComponent(report)}`}
          >
            Save JSON report
          </a>
        </section>
      )}
      <footer>One policy. Your hardware’s capabilities.</footer>
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
