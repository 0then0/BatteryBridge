import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const source = new URL("../../../assets/icon.svg", import.meta.url);
const publicIcon = new URL("../public/icon.svg", import.meta.url);

for (const file of [source, publicIcon]) {
  test(`SVG has a nonempty accessible title: ${file.pathname}`, async () => {
    const svg = await readFile(file, "utf8");
    const title = svg.match(/<title\s+id="([^"]+)"\s*>([^<]+)<\/title>/);
    assert.ok(title?.[2].trim(), "SVG needs a nonempty title");
    assert.ok(svg.includes(`aria-labelledby="${title[1]}"`));
  });
}
test("public favicon stays in sync with the project icon", async () => {
  assert.equal(
    await readFile(source, "utf8"),
    await readFile(publicIcon, "utf8"),
  );
});
