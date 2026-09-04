use rf_rhodes_dsp::{Profile, Voice};

#[test]
fn research_constructor_preserves_default_and_rejects_invalid_steps() {
    for substeps in [0, 1, 2, 3, 5, 63, 65, usize::MAX] {
        assert!(Voice::new_for_convergence(48_000.0, 57, Profile::default(), substeps).is_err());
    }
    let mut normal = Voice::new(48_000.0, 57, Profile::default()).unwrap();
    let mut research = Voice::new_for_convergence(48_000.0, 57, Profile::default(), 4).unwrap();
    normal.strike(0.8);
    research.strike(0.8);
    for _ in 0..20_000 {
        assert_eq!(normal.tick(), research.tick());
    }
}

#[test]
fn refined_contact_stays_passive_and_separates() {
    for substeps in [8, 16, 32, 64] {
        for note in [28, 57, 100] {
            let mut voice =
                Voice::new_for_convergence(48_000.0, note, Profile::default(), substeps).unwrap();
            voice.strike(1.0);
            let mut energy = voice.probe().mechanical_energy_j;
            for _ in 0..(48_000 * substeps / 50) {
                assert!(voice.tick().is_finite());
                let probe = voice.probe();
                assert!(probe.mechanical_energy_j <= energy * (1.0 + 1e-8) + 1e-15);
                energy = probe.mechanical_energy_j;
            }
            assert!(!voice.probe().contact_active);
        }
    }
}
