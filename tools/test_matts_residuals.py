import unittest
from matts_residuals import analyze, cells


def row(note, errors):
    return {"note": note, "layer": "p", "harmonic_terms": len(errors),
            "harmonic_mse_db2": sum(e * e for e in errors) / len(errors),
            "balances": [{"window": i + 1, "harmonic": 2, "error_db": e,
                          "reference_db": -20, "candidate_db": -20 + e} for i, e in enumerate(errors)]}


class ResidualTests(unittest.TestCase):
    def test_partial_coverage_preserves_equal_case_weight(self):
        rows = [row(43, [2, 2]), row(47, [4])]
        result = analyze(rows, rows)["groups"]
        self.assertAlmostEqual(result["all"]["contribution_db2"], 10)
        self.assertAlmostEqual(result["note-43"]["contribution_db2"], 2)
        self.assertAlmostEqual(result["note-47"]["contribution_db2"], 8)

    def test_signed_temporal_error_and_additive_delta(self):
        result = analyze([row(43, [-2, 4])], [row(43, [1, 1])])
        self.assertEqual(result["temporal_pairs"][0]["drift_error_db"], 6)
        self.assertAlmostEqual(result["groups"]["all"]["delta_contribution_db2"], 9)
        self.assertAlmostEqual(sum(r["delta_contribution_db2"] for r in result["largest_regressions"]), 9)

    def test_missing_candidate_uses_floor(self):
        sample = row(43, [-40])
        sample["balances"][0]["candidate_db"] = None
        self.assertTrue(next(iter(cells([sample]).values()))["floor_affected"])

    def test_inconsistent_scores_and_coverage_fail(self):
        with self.assertRaises(ValueError):
            analyze([row(43, [1])], [row(43, [1, 2])])
        invalid = row(43, [1])
        invalid["harmonic_mse_db2"] = 0
        with self.assertRaises(ValueError):
            cells([invalid])


if __name__ == "__main__":
    unittest.main()
