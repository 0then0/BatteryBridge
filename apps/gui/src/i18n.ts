import de from "./locales/de.json" with { type: "json" };
import en from "./locales/en.json" with { type: "json" };
import es from "./locales/es.json" with { type: "json" };
import fr from "./locales/fr.json" with { type: "json" };
import ja from "./locales/ja.json" with { type: "json" };
import ptBR from "./locales/pt-BR.json" with { type: "json" };
import ru from "./locales/ru.json" with { type: "json" };

export type Message = keyof typeof en;
type Catalog = Record<Message, string>;
export const catalogs = {
  en,
  ru,
  es,
  de,
  fr,
  ja,
  "pt-BR": ptBR,
} satisfies Record<string, Catalog>;
export type Locale = keyof typeof catalogs;
export type LanguageChoice = Locale | "system";
export const languageNames: Record<Locale, string> = {
  en: "English",
  ru: "Русский",
  es: "Español",
  de: "Deutsch",
  fr: "Français",
  ja: "日本語",
  "pt-BR": "Português (Brasil)",
};
export const languageStorageKey = "batterybridge.language";

export function isLanguageChoice(value: unknown): value is LanguageChoice {
  return (
    value === "system" ||
    (typeof value === "string" && Object.hasOwn(catalogs, value))
  );
}

export function resolveLocale(
  choice: LanguageChoice,
  preferred: readonly string[],
): Locale {
  if (choice !== "system") return choice;
  for (const tag of preferred) {
    const language = tag.toLowerCase().split(/[-_]/)[0];
    if (language === "pt") return "pt-BR";
    if (Object.hasOwn(catalogs, language)) return language as Locale;
  }
  return "en";
}

export function readLanguage(
  storage: Pick<Storage, "getItem">,
): LanguageChoice {
  try {
    const stored = storage.getItem(languageStorageKey);
    return isLanguageChoice(stored) ? stored : "system";
  } catch {
    return "system";
  }
}

export function translate(
  locale: Locale,
  key: Message,
  values: Record<string, string | number> = {},
): string {
  return catalogs[locale][key].replace(/\{(\w+)\}/g, (token, name: string) =>
    Object.hasOwn(values, name) ? String(values[name]) : token,
  );
}

// These are display translations of known protocol diagnostics. Unknown OS or
// firmware messages remain verbatim; reports always retain the original data.
export const reasonMessages: Record<string, Message> = {
  "no compatible charge-control interface detected": "reasonNone",
  "SMC binary gate; daemon enforces hysteresis while awake; firmware may deny writes":
    "reasonSmc",
  "daemon temporarily releases gate until full": "reasonSmcTopUp",
  "SMC CH0I; daemon restores adapter at target": "reasonSmcDischarge",
  "kernel end threshold; writes require device permissions": "reasonKernelEnd",
  "kernel end threshold": "reasonKernelEnd",
  "kernel start threshold": "reasonKernelStart",
  "temporary 100% threshold, restored by daemon": "reasonKernelTopUp",
  "not exposed by generic threshold backend": "reasonGeneric",
  "Lenovo WMI conservation mode, fixed 80%; device write compatibility is unverified":
    "reasonLenovo",
};

export function capabilityReason(locale: Locale, reason: string): string {
  return Object.hasOwn(reasonMessages, reason)
    ? translate(locale, reasonMessages[reason])
    : reason;
}

export function batteryState(locale: Locale, state: string): string {
  const states: Record<string, Message> = {
    charging: "charging",
    discharging: "discharging",
    full: "full",
    "not charging": "notCharging",
    "not-charging": "notCharging",
    low: "low",
    critical: "critical",
    unknown: "unknown",
  };
  const key = state.toLowerCase();
  return Object.hasOwn(states, key)
    ? translate(locale, states[key])
    : translate(locale, "unknown");
}
