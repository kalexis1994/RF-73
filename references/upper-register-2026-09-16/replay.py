"""Verify stored constraints, selection and lower-register invariance."""
import json
import math
from pathlib import Path

root = Path(__file__).resolve().parent
seed = json.loads((root / "coarse/trial-00.json").read_text())["cases"]
results = []
for stage in ("coarse", "refined"):
    directory = root / stage
    base = json.loads((directory / "development-baseline.json").read_text())["cases"]
    search = json.loads((directory / "search.json").read_text())
    best_key, best = None, None
    for entry in search["history"]:
        trial = json.loads((directory / f"trial-{entry['trial']:02}.json").read_text())
        rows = trial["cases"]
        assert [(r["note"], r["layer"]) for r in rows] == [(r["note"], r["layer"]) for r in base]
        for r, old in zip(rows, seed):
            if r["note"] <= 55:
                assert r == old
            assert math.isclose(r["harmonic_mse_db2"], sum(b["error_db"]**2 for b in r["balances"]) / len(r["balances"]), rel_tol=1e-12)
        ratios = []
        for note in sorted({r["note"] for r in base}):
            ratios.append(sum(r["harmonic_mse_db2"] for r in rows if r["note"] == note) / sum(r["harmonic_mse_db2"] for r in base if r["note"] == note))
        ratios.extend(r["harmonic_mse_db2"] / b["harmonic_mse_db2"] for r, b in zip(rows, base) if (r["note"], r["layer"]) in ((47, "mp"), (64, "f")))
        mean = sum(r["harmonic_mse_db2"] for r in rows) / sum(r["harmonic_mse_db2"] for r in base)
        violation = max(max(ratios)/1.1, mean/0.9)
        assert math.isclose(violation, trial["assessment"]["constraint_violation"], rel_tol=1e-12)
        key = (0, mean) if violation <= 1 else (1, violation)
        if best_key is None or key < best_key:
            best_key, best = key, entry
    assert best["coordinates"] == search["winner_coordinates"]
    results.append({"stage": stage, "trials": len(search["history"]), "winner_trial": best["trial"], "winner_coordinates": best["coordinates"], "verified": True})
print(json.dumps({"selection_and_lower_register_invariance": results}, indent=2))
