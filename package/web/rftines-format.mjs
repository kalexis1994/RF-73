const FORMAT = "rackforge.rftines-program";
const PLUGIN_ID = "org.rackforge.rftines";
const MAX_FILE_BYTES = 32 * 1024;

const CORE_FIELDS = [
  "gain",
  "law",
  "distance_mm",
  "alignment_mm",
  "hardness",
  "sustain",
  "bell",
  "dynamics",
];
const ELECTRONICS_FIELDS = [
  "bass_db",
  "treble_db",
  "vibrato",
  "speed_hz",
  "intensity",
  "preamp",
  "bass_boost",
];
const PROGRAM_FIELDS = [
  "schema_version",
  "id",
  "name",
  "plugin_id",
  "plugin_version",
  "plugin_state_version",
  "payload_version",
  "category",
  "tags",
  "payload",
];

function object(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function exactKeys(value, allowed, required = allowed) {
  if (!object(value)) return false;
  const keys = Object.keys(value);
  return keys.every((key) => allowed.includes(key)) && required.every((key) => keys.includes(key));
}

function finiteRange(value, minimum, maximum) {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

function printable(value, maximum = 64) {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    value.length <= maximum &&
    !/[\p{Cc}\p{Cf}]/u.test(value)
  );
}

function identifier(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 64 &&
    !value.startsWith(".") &&
    !value.endsWith(".") &&
    !value.includes("..") &&
    /^[a-z0-9._-]+$/.test(value)
  );
}

export function canonicalJson(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (object(value)) {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

async function digest(value) {
  const bytes = new TextEncoder().encode(canonicalJson(value));
  const hash = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(hash), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export function validateProgram(program) {
  const required = PROGRAM_FIELDS.filter((field) => !["category", "tags"].includes(field));
  if (!exactKeys(program, PROGRAM_FIELDS, required)) throw new Error("Unknown or missing program fields.");
  if (program.schema_version !== 1 || program.payload_version !== 1) {
    throw new Error("Unsupported program schema.");
  }
  if (program.plugin_id !== PLUGIN_ID) throw new Error("This file belongs to another plugin.");
  if (![4, 5].includes(program.plugin_state_version)) {
    throw new Error("Unsupported RF-Tines program version.");
  }
  if (!identifier(program.id)) throw new Error("The program identifier is invalid.");
  if (!printable(program.name)) throw new Error("The program name must contain 1-64 printable characters.");
  if (typeof program.plugin_version !== "string" || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(program.plugin_version)) {
    throw new Error("The plugin version is invalid.");
  }
  if (program.category !== undefined && !printable(program.category)) {
    throw new Error("The program category is invalid.");
  }
  if (program.tags !== undefined) {
    if (!Array.isArray(program.tags) || program.tags.length > 16) throw new Error("The program tags are invalid.");
    const tags = new Set();
    for (const tag of program.tags) {
      if (!printable(tag) || tags.has(tag.toLowerCase())) throw new Error("The program tags are invalid.");
      tags.add(tag.toLowerCase());
    }
  }
  const requiredPayload = program.plugin_state_version === 5
    ? [...CORE_FIELDS, ...ELECTRONICS_FIELDS]
    : CORE_FIELDS;
  if (!exactKeys(program.payload, [...CORE_FIELDS, ...ELECTRONICS_FIELDS], requiredPayload)) {
    throw new Error("Unknown or missing RF-Tines parameters.");
  }
  const p = program.payload;
  const valid =
    finiteRange(p.gain, 0, 2) &&
    Number.isInteger(p.law) && finiteRange(p.law, 0, 2) &&
    finiteRange(p.distance_mm, 0.5, 3) &&
    finiteRange(p.alignment_mm, -1, 1.5) &&
    finiteRange(p.hardness, 0, 1) &&
    finiteRange(p.sustain, 0, 1) &&
    finiteRange(p.bell, 0, 1) &&
    finiteRange(p.dynamics, 0, 1) &&
    (p.bass_db === undefined || finiteRange(p.bass_db, -12, 12)) &&
    (p.treble_db === undefined || finiteRange(p.treble_db, -12, 12)) &&
    (p.vibrato === undefined || [0, 1].includes(p.vibrato)) &&
    (p.speed_hz === undefined || finiteRange(p.speed_hz, 0.5, 12)) &&
    (p.intensity === undefined || finiteRange(p.intensity, 0, 1)) &&
    (p.preamp === undefined || [0, 1].includes(p.preamp)) &&
    (p.bass_boost === undefined || finiteRange(p.bass_boost, 0, 1));
  if (!valid) throw new Error("One or more RF-Tines parameters are outside their valid range.");
  return structuredClone(program);
}

export async function createRfTinesFile(program) {
  const checked = validateProgram(program);
  const file = {
    format: FORMAT,
    schema_version: 1,
    program: checked,
    integrity: { algorithm: "SHA-256", digest: await digest(checked) },
  };
  return `${JSON.stringify(file, null, 2)}\n`;
}

export async function parseRfTinesFile(text) {
  if (typeof text !== "string" || new TextEncoder().encode(text).byteLength > MAX_FILE_BYTES) {
    throw new Error("The .rftines file is larger than 32 KiB.");
  }
  let file;
  try {
    file = JSON.parse(text);
  } catch {
    throw new Error("The .rftines file is not valid JSON.");
  }
  if (!exactKeys(file, ["format", "schema_version", "program", "integrity"])) {
    throw new Error("Unknown or missing .rftines file fields.");
  }
  if (file.format !== FORMAT || file.schema_version !== 1) throw new Error("Unsupported .rftines file format.");
  if (!exactKeys(file.integrity, ["algorithm", "digest"]) || file.integrity.algorithm !== "SHA-256") {
    throw new Error("Unsupported .rftines integrity record.");
  }
  if (!/^[0-9a-f]{64}$/.test(file.integrity.digest)) throw new Error("The .rftines checksum is invalid.");
  const program = validateProgram(file.program);
  if ((await digest(program)) !== file.integrity.digest) throw new Error("The .rftines file is damaged or was modified.");
  return program;
}

export const RFTINES_MAX_FILE_BYTES = MAX_FILE_BYTES;
