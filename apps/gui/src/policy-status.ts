import type { Snapshot } from "./api";

// Saved settings are intent, not proof of hardware enforcement. In particular,
// private SMC gates may reset while the computer sleeps.
export function chargingAboveLimit(snapshot: Snapshot | null): boolean {
  const stop = snapshot?.config?.charge_limit;
  const battery = snapshot?.batteries[0];
  return (
    stop != null &&
    !snapshot?.top_up_active &&
    snapshot?.discharge_target == null &&
    battery?.percent != null &&
    battery.percent > stop &&
    battery.on_ac === true &&
    battery.state.toLowerCase() === "charging"
  );
}
