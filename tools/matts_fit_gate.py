"""Evaluate frozen Rust receipts; does not process audio or tune parameters."""
import argparse
import hashlib
import json
import math
from pathlib import Path


def evaluate(reports):
    reasons = []
    reductions = {}
    note_results = []
    eligible = []
    newly_failed = []
    missing = []
    for split in ("training", "held_out"):
        base = reports[f"{split}-baseline"]
        candidate = reports[f"{split}-candidate"]
        old = base["harmonic_mse_db2"]
        new = candidate["harmonic_mse_db2"]
        if not (math.isfinite(old) and math.isfinite(new) and old > 0 and new >= 0):
            raise ValueError("Invalid aggregate objective")
        reductions[split] = 1 - new / old
        if reductions[split] < 0.1:
            reasons.append(f"{split}_harmonic_reduction_below_10_percent")
        key = lambda row: (row["note"], row["layer"])
        baseline_rows = {key(row): row for row in base["cases"]}
        candidate_rows = {key(row): row for row in candidate["cases"]}
        if len(baseline_rows) != len(base["cases"]) or len(candidate_rows) != len(candidate["cases"]):
            raise ValueError("Duplicate case")
        if baseline_rows.keys() != candidate_rows.keys():
            raise ValueError("Mismatched evaluation cases")
        unscored = [k for k in baseline_rows if baseline_rows[k]["harmonic_mse_db2"] is None
                    or candidate_rows[k]["harmonic_mse_db2"] is None]
        if unscored:
            reasons.append(f"{split}_incomplete_harmonic_coverage")
        if split != "held_out":
            continue
        for note in sorted({k[0] for k in baseline_rows}):
            keys = [k for k in baseline_rows if k[0] == note and k not in unscored]
            if not keys:
                note_results.append({"note": note, "scored_cases": 0})
                continue
            a = sum(baseline_rows[k]["harmonic_mse_db2"] for k in keys) / len(keys)
            b = sum(candidate_rows[k]["harmonic_mse_db2"] for k in keys) / len(keys)
            note_results.append({"note": note, "scored_cases": len(keys), "baseline_mse_db2": a, "candidate_mse_db2": b})
            if b > a * 1.1:
                reasons.append(f"held_out_note_{note}_regression_above_10_percent")
        for k, row in baseline_rows.items():
            ref = row.get("reference_envelope", {})
            a = row.get("candidate_envelope", {})
            b = candidate_rows[k].get("candidate_envelope", {})
            if not ref.get("qualified") or not a.get("qualified"):
                missing.append({"note": k[0], "layer": k[1],
                                "reference_qualified": bool(ref.get("qualified")),
                                "baseline_qualified": bool(a.get("qualified"))})
                continue
            if not b.get("qualified"):
                newly_failed.append({"note": k[0], "layer": k[1],
                                     "reasons": b.get("rejection_reasons", [])})
                continue
            rate = lambda item: -20 / math.log(10) * item["provisional_fit"]["amplitude_decay_per_second"]
            r, av, bv = rate(ref), rate(a), rate(b)
            eligible.append({"note": k[0], "layer": k[1], "reference_db_s": r,
                             "baseline_db_s": av, "candidate_db_s": bv,
                             "baseline_absolute_error": abs(av-r), "candidate_absolute_error": abs(bv-r)})
    if newly_failed:
        reasons.append("newly_failed_held_out_envelope_gates")
    slope_delta = None
    if eligible:
        slope_delta = sum(r["candidate_absolute_error"]-r["baseline_absolute_error"] for r in eligible) / len(eligible)
        if slope_delta > 0.5:
            reasons.append("held_out_envelope_error_increase_above_0_5_db_s")
    else:
        reasons.append("no_common_qualified_held_out_envelopes")
    return {"promote": not reasons, "rejection_reasons": reasons,
            "harmonic_mse_reductions": reductions, "held_out_notes": note_results,
            "envelope_error_increase_db_s": slope_delta, "qualified_envelope_comparisons": eligible,
            "newly_failed_envelopes": newly_failed, "excluded_envelope_cases": missing}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt_directory", type=Path)
    parser.add_argument("new_output", type=Path)
    args = parser.parse_args()
    reports = {f"{split}-{name}": json.loads((args.receipt_directory / f"{split}-{name}.json").read_text())
               for split in ("training", "held_out") for name in ("baseline", "candidate")}
    result = evaluate(reports)
    result["receipt_hashes"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                                for p in sorted(args.receipt_directory.glob("*.json"))}
    with args.new_output.open("x", encoding="utf-8") as handle:
        json.dump(result, handle, indent=2, allow_nan=False)
        handle.write("\n")
    print(json.dumps({k: result[k] for k in ("promote", "rejection_reasons", "harmonic_mse_reductions")}, indent=2))


if __name__ == "__main__":
    main()
