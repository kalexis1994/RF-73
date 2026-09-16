"""Decompose retained fit receipts without loading audio or fitting parameters."""
import argparse
import hashlib
import json
import math
from collections import defaultdict
from pathlib import Path


def cells(rows):
    """Preserve the scorer's equal-case weighting, including partial coverage."""
    if not rows:
        raise ValueError("No scored cases")
    result = {}
    for row in rows:
        terms = row["balances"]
        if not terms or row["harmonic_terms"] != len(terms):
            raise ValueError("Missing or inconsistent harmonic coverage")
        measured = sum(t["error_db"] ** 2 for t in terms) / len(terms)
        if not math.isclose(measured, row["harmonic_mse_db2"], rel_tol=1e-10, abs_tol=1e-10):
            raise ValueError("Stored score does not match retained residuals")
        for term in terms:
            key = (row["note"], row["layer"], term["window"], term["harmonic"])
            if key in result:
                raise ValueError("Duplicate spectral cell")
            error = term["error_db"]
            if not math.isfinite(error):
                raise ValueError("Nonfinite residual")
            reference = max(-60.0, term["reference_db"])
            candidate = term["candidate_db"]
            predicted = -60.0 if candidate is None else max(-60.0, candidate)
            if not math.isclose(predicted - reference, error, abs_tol=1e-10):
                raise ValueError("Residual does not match the scoring floor")
            result[key] = {
                "error_db": error, "weight": 1 / (len(rows) * len(terms)),
                "reference_db": reference, "candidate_db": predicted,
                "floor_affected": candidate is None or candidate <= -60 or term["reference_db"] <= -60,
            }
    return result


def analyze(rows, base_rows):
    current, base = cells(rows), cells(base_rows)
    if current.keys() != base.keys():
        raise ValueError("Cannot compare different spectral coverage")
    groups = defaultdict(list)
    for key, value in current.items():
        note, layer, window, harmonic = key
        for group in ("all", f"H{harmonic}", f"window-{window}", f"window-{window}/H{harmonic}",
                      f"note-{note}", f"note-{note}/H{harmonic}", f"layer-{layer}"):
            groups[group].append((key, value))
    summary = {}
    for group, entries in sorted(groups.items()):
        weight = sum(v["weight"] for _, v in entries)
        cost = sum(v["weight"] * v["error_db"] ** 2 for _, v in entries)
        baseline_cost = sum(base[k]["weight"] * base[k]["error_db"] ** 2 for k, _ in entries)
        summary[group] = {
            "cells": len(entries), "objective_weight": weight,
            "contribution_db2": cost, "baseline_contribution_db2": baseline_cost,
            "delta_contribution_db2": cost - baseline_cost,
            "weighted_bias_db": sum(v["weight"] * v["error_db"] for _, v in entries) / weight,
            "weighted_rmse_db": math.sqrt(cost / weight),
            "floor_affected_cells": sum(v["floor_affected"] for _, v in entries),
        }
    ranked = []
    temporal = []
    for key, value in current.items():
        note, layer, window, harmonic = key
        ranked.append({"note": note, "layer": layer, "window": window, "harmonic": harmonic,
                       **value, "delta_contribution_db2": value["weight"] * value["error_db"] ** 2
                       - base[key]["weight"] * base[key]["error_db"] ** 2})
        body = current.get((note, layer, 2, harmonic))
        if window == 1 and body is not None:
            temporal.append({"note": note, "layer": layer, "harmonic": harmonic,
                             "reference_body_minus_attack_db": body["reference_db"] - value["reference_db"],
                             "candidate_body_minus_attack_db": body["candidate_db"] - value["candidate_db"],
                             "drift_error_db": body["error_db"] - value["error_db"],
                             "floor_affected": body["floor_affected"] or value["floor_affected"]})
    expected = sum(r["harmonic_mse_db2"] for r in rows) / len(rows)
    assert math.isclose(summary["all"]["contribution_db2"], expected, rel_tol=1e-12)
    return {"groups": summary, "largest_regressions": sorted(ranked, key=lambda r: -r["delta_contribution_db2"]),
            "temporal_pairs": temporal}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="New JSON file; paths are relative to the workspace")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    paths = {
        "fixed_calibrated": "references/pickup-family-screen-2026-09-15/development-fixed-baseline.json",
        "remapped_calibrated": "references/velocity-mapping-2026-09-15/fine/calibrated-selection.json",
        "smooth_register": "references/velocity-mapping-2026-09-15/fine/register-selection.json",
        "production": "references/pickup-family-screen-2026-09-15/production-gap-500-speed-160.json",
        "point_pole": "references/pickup-family-screen-2026-09-15/point-pole-gap-1000-speed-80.json",
    }
    loaded = {name: json.loads((root / path).read_text()) for name, path in paths.items()}
    rows = {name: value.get("cases", value.get("selected_cases")) for name, value in loaded.items()}
    pitch_path = "references/continuous-pickup-fit-2026-09-15/training-baseline.json"
    pitch_rows = json.loads((root / pitch_path).read_text())["cases"]
    source_keys = {(r["note"], r["layer"]) for r in rows["fixed_calibrated"]}
    pitch_keys = {(r["note"], r["layer"]) for r in pitch_rows}
    if not pitch_keys <= source_keys or {key for key in source_keys if key[0] == 72} - pitch_keys:
        raise ValueError("Pitch evidence does not cover C5 within development cases")
    coverage = []
    for row in rows["fixed_calibrated"]:
        observed = {(t["window"], t["harmonic"]) for t in row["balances"]}
        coverage.append({"note": row["note"], "layer": row["layer"], "terms": len(observed),
                         "missing_reference_terms": [[w, h] for w in (1, 2) for h in (2, 3, 4) if (w, h) not in observed]})
    result = {
        "scope": "Development receipts only; no audio loaded, parameters fitted, or held-out observations inspected",
        "windows": {"1": "attack96", "2": "body350 at 250 ms"},
        "units": "Contributions are additive terms of the equal-case mean squared dB objective; bias/RMSE are within-group",
        "inputs": {name: {"path": path, "sha256_lf": hashlib.sha256((root / path).read_bytes().replace(b'\r\n', b'\n')).hexdigest()} for name, path in paths.items()},
        "coverage": coverage,
        "pitch_evidence": {"path": pitch_path, "sha256_lf": hashlib.sha256((root / pitch_path).read_bytes().replace(b'\r\n', b'\n')).hexdigest(),
                           "c5": [{"layer": r["layer"], "anchor": r["reference_pitch_anchor"]} for r in pitch_rows if r["note"] == 72]},
        "models": {name: analyze(value, rows["fixed_calibrated"]) for name, value in rows.items()},
    }
    with args.output.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
    print(f"Wrote residual decomposition for {len(rows)} models to {args.output}")


if __name__ == "__main__":
    main()
