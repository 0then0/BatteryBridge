import { invoke } from "@tauri-apps/api/core";
// Wire DTOs mirror batterybridge-protocol v2. Rust validates every incoming command.
export interface Capability {
  state: "supported" | "experimental" | "unsupported" | "unknown";
  reason: string;
}
export interface Capabilities {
  charge_limit: Capability;
  start_threshold: Capability;
  stop_threshold: Capability;
  force_discharge: Capability;
  adapter_disable: Capability;
  temporary_full_charge: Capability;
  allowed_limits: number[] | null;
}
export interface Battery {
  id: string;
  percent: number | null;
  state: string;
  on_ac: boolean | null;
  health_percent: number | null;
  hardware_stop: number | null;
}
export interface Snapshot {
  batteries: Battery[];
  backend: string;
  device: { vendor: string; model: string };
  capabilities: Capabilities;
  config: {
    charge_limit: number | null;
    start_threshold: { enabled: boolean; value: number };
  } | null;
  top_up_active: boolean;
  discharge_target: number | null;
  last_error: unknown;
}
export type Command =
  | { command: "status" | "probe" | "top_up" }
  | { command: "limit"; stop: number | null; start: number | null };
type Response =
  | { type: "status"; data: Snapshot }
  | { type: "probe"; data: unknown }
  | { type: "ok" }
  | { type: "error"; data: unknown };
export function request(command: Command): Promise<Response> {
  return invoke<Response>("request", { command });
}
export function describeError(
  error: unknown,
  fallback = "The BatteryBridge service could not complete this request.",
): string {
  if (typeof error === "string") return error;
  if (
    typeof error === "object" &&
    error !== null &&
    "detail" in error &&
    typeof error.detail === "string"
  )
    return error.detail;
  return fallback;
}

export function setTrayLanguage(locale: string): Promise<void> {
  return invoke("set_language", { locale });
}

export function available(capability?: Capability): boolean {
  return (
    capability?.state === "supported" || capability?.state === "experimental"
  );
}
