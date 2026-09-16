"""Independently replay selection from retained costs, without processing audio."""
import itertools
import json
import math
from pathlib import Path


def replay(directory):
    base = json.loads((directory / "development-fixed-baseline.json").read_text())["cases"]
    layers = {name: i for i, name in enumerate(("p", "mp", "mf", "f"))}
    notes = sorted({r["note"] for r in base})
    groups = [[i for i, r in enumerate(base) if r["note"] == n] for n in notes]
    groups += [[i] for i, r in enumerate(base) if (r["note"], r["layer"]) in ((47, "mp"), (64, "f"))]
    denominators = [sum(base[i]["harmonic_mse_db2"] for i in group) for group in groups]
    total = sum(r["harmonic_mse_db2"] for r in base)
    results = []
    for path in sorted(directory.glob("*-gap-*-speed-*.json")):
        receipt = json.loads(path.read_text())
        if receipt.get("status") == "not_evaluable":
            assert receipt["evaluated_mappings"] == 0
            results.append({"id": path.stem, "status": "not_evaluable"})
            continue
        grid = receipt["grid"]
        best_key, best_mapping = None, None
        count = 0
        for indices in itertools.combinations(range(len(grid)), 4):
            costs = [grid[indices[layers[r["layer"]]]]["cases"][i]["harmonic_mse_db2"] for i, r in enumerate(base)]
            mean = sum(costs) / total
            worst = max(sum(costs[i] for i in group) / denom for group, denom in zip(groups, denominators))
            violation = max(worst / 1.1, mean / 0.9)
            key = (0, mean) if violation <= 1 else (1, violation)
            if best_key is None or key < best_key:
                best_key = key
                best_mapping = [grid[i]["velocity"] for i in indices]
                best_values = (mean, violation)
            count += 1
        assert best_mapping == receipt["mapping"], path
        assert count == receipt["evaluated_mappings"] == 3876
        assert math.isclose(best_values[0], receipt["assessment"]["mean_ratio"], rel_tol=1e-12)
        assert math.isclose(best_values[1], receipt["assessment"]["constraint_violation"], rel_tol=1e-12)
        results.append({"id": path.stem, "mapping": best_mapping, "evaluated_mappings": count, "matches": True})
    return {"configurations": results, "total_mapping_evaluations": sum(r.get("evaluated_mappings", 0) for r in results)}


if __name__ == "__main__":
    import sys
    folder = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).parent
    print(json.dumps(replay(folder), indent=2))
