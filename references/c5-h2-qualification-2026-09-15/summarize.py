"""Summarize probes and an explicitly post-hoc, frozen-fit sensitivity check."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
observations = json.loads((HERE / "observations.json").read_text())["observations"]
groups = []
excluded = set()
for layer in ("p", "mp", "mf", "f"):
    for window in ("attack", "body"):
        rows = [r for r in observations if r["layer"] == layer and r["window"] == window]
        assert len(rows) == 9
        ranges = {}
        for key in ("relative_to_h1_db", "median_margin_db", "p90_margin_db", "frequency_hz"):
            values = [r["probe"][key] for r in rows if r["probe"][key] is not None]
            ranges[key] = [min(values), max(values)] if values else None
        groups.append({"layer": layer, "window": window, "supported": sum(r["probe"]["supported"] for r in rows), "ranges": ranges})
        offset, duration = (0.0, 0.096) if window == "attack" else (0.25, 0.35)
        nominal = [r for r in rows if r["offset_seconds"] == offset and r["probe"]["duration_seconds"] == duration]
        assert len(nominal) == 1
        if not nominal[0]["probe"]["supported"]:
            excluded.add((72, layer, 1 if window == "attack" else 2, 2))

inputs = json.loads((ROOT / "references/pickup-family-screen-2026-09-15/residual-decomposition.json").read_text())["inputs"]
scores = {}
for name, info in inputs.items():
    receipt = json.loads((ROOT / info["path"]).read_text())
    cases = receipt.get("cases", receipt.get("selected_cases"))
    values = {}
    for case in cases:
        terms = [t for t in case["balances"] if (case["note"], case["layer"], t["window"], t["harmonic"]) not in excluded]
        assert terms
        values[(case["note"], case["layer"])] = sum(t["error_db"]**2 for t in terms) / len(terms)
    scores[name] = values
base = scores["fixed_calibrated"]
sensitivity = []
for name, values in scores.items():
    notes = {note: sum(v for (n, _), v in values.items() if n == note) / sum(v for (n, _), v in base.items() if n == note) for note in sorted({n for n, _ in base})}
    critical = {f"{n}/{layer}": values[(n, layer)] / base[(n, layer)] for n, layer in ((47, "mp"), (64, "f"))}
    ratio = sum(values.values()) / sum(base.values())
    worst = max(*notes.values(), *critical.values())
    sensitivity.append({"model": name, "mean_ratio": ratio, "worst_ratio": worst,
                        "satisfies_existing_numeric_limits": ratio <= 0.9 and worst <= 1.1,
                        "note_ratios": notes, "critical_ratios": critical})
print(json.dumps({"groups": groups, "post_hoc_sensitivity": {
    "scope": "Remove unsupported nominal C5 H2 cells from both frozen candidate and baseline, renormalizing within each case. Not a new fit, revised gate, or promotion decision.",
    "excluded_cells": sorted(excluded), "models": sensitivity}}, indent=2))
