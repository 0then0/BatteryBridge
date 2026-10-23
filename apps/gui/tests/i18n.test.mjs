import assert from "node:assert/strict";
import test from "node:test";
import {
  batteryState,
  capabilityReason,
  catalogs,
  isLanguageChoice,
  readLanguage,
  resolveLocale,
  translate,
} from "../src/i18n.ts";

test("every language has complete messages and matching interpolation tokens", () => {
  const keys = Object.keys(catalogs.en).sort();
  for (const [locale, catalog] of Object.entries(catalogs)) {
    assert.deepEqual(Object.keys(catalog).sort(), keys, locale);
    for (const key of keys) {
      assert.ok(catalog[key].trim(), `${locale}.${key}`);
      assert.deepEqual(
        catalog[key].match(/\{\w+\}/g),
        catalogs.en[key].match(/\{\w+\}/g),
        `${locale}.${key}`,
      );
    }
  }
});

test("system choice respects ordered regional preferences and falls back to English", () => {
  for (const [preferences, expected] of [
    [["ru-RU", "en-US"], "ru"],
    [["es-MX"], "es"],
    [["de-AT"], "de"],
    [["fr-CA"], "fr"],
    [["ja-JP"], "ja"],
    [["pt-BR"], "pt-BR"],
    [["pt-PT"], "pt-BR"],
    [["zh-CN", "fr-FR", "de-DE"], "fr"],
    [["en-GB", "ru-RU"], "en"],
    [["zh-CN"], "en"],
    [[], "en"],
  ]) {
    assert.equal(resolveLocale("system", preferences), expected);
  }
  assert.equal(resolveLocale("de", ["ru-RU"]), "de");
});

test("invalid or inaccessible saved preferences use system language", () => {
  for (const stored of [null, "", "constructor", "__proto__", "xx", "RU"]) {
    assert.equal(readLanguage({ getItem: () => stored }), "system");
  }
  assert.equal(readLanguage({ getItem: () => "pt-BR" }), "pt-BR");
  assert.equal(
    readLanguage({
      getItem: () => {
        throw new Error("Storage denied");
      },
    }),
    "system",
  );
  assert.equal(isLanguageChoice("toString"), false);
});

test("interpolation updates values without modifying translation catalogs", () => {
  for (const locale of Object.keys(catalogs)) {
    assert.ok(translate(locale, "activeLimit", { percent: 85 }).includes("85"));
    assert.ok(catalogs[locale].activeLimit.includes("{percent}"));
  }
});

test("known capability explanations translate but unknown diagnostics stay verbatim", () => {
  const original = "daemon temporarily releases gate until full";
  assert.equal(capabilityReason("ru", original), catalogs.ru.reasonSmcTopUp);
  assert.equal(original, "daemon temporarily releases gate until full");
  const technical = "permission denied: AppleSMC (os error 13)";
  assert.equal(capabilityReason("ja", technical), technical);
  assert.equal(capabilityReason("ru", "constructor"), "constructor");
});

test("battery states from all three platforms have localized display values", () => {
  for (const locale of Object.keys(catalogs)) {
    assert.equal(batteryState(locale, "Charging"), catalogs[locale].charging);
    assert.equal(
      batteryState(locale, "not charging"),
      catalogs[locale].notCharging,
    );
    assert.equal(
      batteryState(locale, "Not-charging"),
      catalogs[locale].notCharging,
    );
    assert.equal(batteryState(locale, "full"), catalogs[locale].full);
    assert.equal(batteryState(locale, "low"), catalogs[locale].low);
    assert.equal(batteryState(locale, "critical"), catalogs[locale].critical);
    assert.equal(
      batteryState(locale, "future state"),
      catalogs[locale].unknown,
    );
  }
});
