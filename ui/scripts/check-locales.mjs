import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const localesDir = join(__dirname, "..", "src", "locales");

const requiredLocales = [
  "en",
  "zh-Hans",
  "zh-Hant",
  "ja",
  "ko",
  "fr",
  "de",
  "es",
  "ru",
  "pt",
  "it",
  "ar",
];

function readKeys(locale) {
  const source = readFileSync(join(localesDir, `${locale}.ftl`), "utf8");
  const keys = new Set();
  for (const line of source.split(/\r?\n/)) {
    const match = /^([A-Za-z][A-Za-z0-9-]*)\s*=/.exec(line);
    if (match) keys.add(match[1]);
  }
  return keys;
}

const files = new Set(
  readdirSync(localesDir)
    .filter((file) => file.endsWith(".ftl"))
    .map((file) => file.replace(/\.ftl$/, "")),
);

const missingFiles = requiredLocales.filter((locale) => !files.has(locale));
if (missingFiles.length > 0) {
  console.error(`Missing locale files: ${missingFiles.join(", ")}`);
  process.exitCode = 1;
}

const reference = readKeys("en");
for (const locale of requiredLocales.filter((item) => files.has(item))) {
  const keys = readKeys(locale);
  const missing = [...reference].filter((key) => !keys.has(key));
  const extra = [...keys].filter((key) => !reference.has(key));
  if (missing.length > 0 || extra.length > 0) {
    console.error(`Locale ${locale} key mismatch`);
    if (missing.length > 0) console.error(`  Missing: ${missing.join(", ")}`);
    if (extra.length > 0) console.error(`  Extra: ${extra.join(", ")}`);
    process.exitCode = 1;
  }
}

if (!process.exitCode) {
  console.log(`Locale check passed for ${requiredLocales.length} locales.`);
}
