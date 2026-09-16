import copy
import unittest
from matts_fit_gate import evaluate


def fixture():
    env = {"qualified": True, "provisional_fit": {"amplitude_decay_per_second": 0.3}}
    rows = [{"note": 55, "layer": "mf", "harmonic_mse_db2": 10.0,
             "reference_envelope": env, "candidate_envelope": env}]
    reports = {}
    for split in ("training", "held_out"):
        for name, score in (("baseline", 10.0), ("candidate", 8.0)):
            reports[f"{split}-{name}"] = {"harmonic_mse_db2": score, "cases": copy.deepcopy(rows)}
            reports[f"{split}-{name}"]["cases"][0]["harmonic_mse_db2"] = score
    return reports


class Gates(unittest.TestCase):
    def test_consistent_improvement_passes(self):
        self.assertTrue(evaluate(fixture())["promote"])

    def test_aggregate_improvement_cannot_hide_note_regression(self):
        reports = fixture()
        reports["held_out-candidate"]["cases"][0]["harmonic_mse_db2"] = 12.0
        result = evaluate(reports)
        self.assertIn("held_out_note_55_regression_above_10_percent", result["rejection_reasons"])
        self.assertFalse(result["promote"])

    def test_failed_envelope_cannot_disappear_from_average(self):
        reports = fixture()
        reports["held_out-candidate"]["cases"][0]["candidate_envelope"]["qualified"] = False
        self.assertFalse(evaluate(reports)["promote"])

    def test_missing_harmonics_do_not_count_as_zero_error(self):
        reports = fixture()
        reports["held_out-candidate"]["cases"][0]["harmonic_mse_db2"] = None
        self.assertIn("held_out_incomplete_harmonic_coverage", evaluate(reports)["rejection_reasons"])

    def test_unknown_reference_cannot_approve_sustain(self):
        reports = fixture()
        del reports["held_out-baseline"]["cases"][0]["reference_envelope"]
        self.assertIn("no_common_qualified_held_out_envelopes", evaluate(reports)["rejection_reasons"])

    def test_mismatched_cases_rejected(self):
        reports = fixture()
        reports["held_out-candidate"]["cases"][0]["note"] = 56
        with self.assertRaises(ValueError):
            evaluate(reports)


if __name__ == "__main__":
    unittest.main()
