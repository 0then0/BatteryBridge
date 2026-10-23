import assert from "node:assert/strict";
import test from "node:test";
import { chargingAboveLimit } from "../src/policy-status.ts";

test("reports observed charging above the saved limit without treating intent as enforcement", () => {
  const snapshot = {
    config: { charge_limit: 80 },
    batteries: [{ percent: 86, state: "Charging", on_ac: true }],
    top_up_active: false,
    discharge_target: null,
  };
  assert.equal(chargingAboveLimit(snapshot), true);
  for (const patch of [
    { config: null },
    { config: { charge_limit: null } },
    { top_up_active: true },
    { discharge_target: 70 },
    { batteries: [] },
    { batteries: [{ percent: 80, state: "charging", on_ac: true }] },
    { batteries: [{ percent: null, state: "charging", on_ac: true }] },
    { batteries: [{ percent: 86, state: "discharging", on_ac: true }] },
    { batteries: [{ percent: 86, state: "charging", on_ac: false }] },
  ]) {
    assert.equal(
      chargingAboveLimit({ ...snapshot, ...patch }),
      false,
      JSON.stringify(patch),
    );
  }
  assert.equal(chargingAboveLimit(null), false);
});
