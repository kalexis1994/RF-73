import assert from "node:assert/strict";
import { test } from "node:test";
import {
  createRfTinesFile,
  parseRfTinesFile,
  validateProgram,
} from "../package/web/rftines-format.mjs";

globalThis.crypto ??= (await import("node:crypto")).webcrypto;

const program = {
  schema_version: 1,
  id: "stage-early-70s",
  name: "Stage 73 - Early '70s",
  plugin_id: "org.rackforge.rftines",
  plugin_version: "0.1.10",
  plugin_state_version: 5,
  payload_version: 1,
  category: "Electric Piano",
  tags: ["custom"],
  payload: {
    gain: 0.1,
    law: 2,
    distance_mm: 0.75,
    alignment_mm: 0.48,
    hardness: 0.42,
    sustain: 0.48,
    bell: 0.22,
    dynamics: 0.5,
    bass_db: 0,
    treble_db: 0,
    vibrato: 0,
    speed_hz: 4,
    intensity: 0,
    preamp: 0,
    bass_boost: 0.9,
  },
};

test("roundtrips a checksummed RF-Tines program", async () => {
  const named = { ...program, name: "Cálido y brillante" };
  const text = await createRfTinesFile(named);
  assert.deepEqual(await parseRfTinesFile(text), named);
});

test("rejects corruption, foreign plugins, unknown fields and invalid ranges", async () => {
  const text = await createRfTinesFile(program);
  await assert.rejects(parseRfTinesFile(text.replace("0.48", "0.49")), /damaged|modified/);
  assert.throws(() => validateProgram({ ...program, plugin_id: "org.example.other" }), /another plugin/);
  assert.throws(() => validateProgram({ ...program, surprise: true }), /Unknown or missing/);
  assert.throws(
    () => validateProgram({ ...program, payload: { ...program.payload, speed_hz: 20 } }),
    /outside their valid range/,
  );
});

test("accepts old neutral-electronics documents", () => {
  const legacy = structuredClone(program);
  legacy.plugin_state_version = 4;
  for (const key of ["bass_db", "treble_db", "vibrato", "speed_hz", "intensity", "preamp", "bass_boost"]) {
    delete legacy.payload[key];
  }
  assert.equal(validateProgram(legacy).plugin_state_version, 4);
});
