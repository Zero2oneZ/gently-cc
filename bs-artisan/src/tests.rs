#[cfg(test)]
mod tests {
    use crate::barf::{barf, BarfQuery, xor_distance};
    use crate::foam::Foam;
    use crate::torus::{AgencyFrame, Torus};
    use std::collections::HashMap;

    fn make_torus(label: &str, count: u32, turn: u32, cooccur_count: usize) -> Torus {
        let mut t = Torus::new(label, 10);
        t.count = count;
        t.last_seen_turn = turn;
        for i in 0..cooccur_count {
            let fake_id: [u8; 32] = blake3::hash(format!("fake{i}").as_bytes()).into();
            t.observe_cooccur(fake_id);
        }
        t
    }

    // ── Torus weight mechanics ────────────────────────────────

    #[test]
    fn torus_weight_decays_with_turn_gap() {
        // cooccur=1 needed: cluster_density = ln(2) > 0, otherwise weight=0 always
        let t = make_torus("auth", 5, 0, 1);
        let w_fresh = t.weight(0);
        let w_stale = t.weight(20);
        assert!(w_stale < w_fresh, "weight should decay as turns pass: {} vs {}", w_stale, w_fresh);
    }

    #[test]
    fn torus_weight_increases_with_cluster() {
        // bare has weight 0 (ln(1)=0), clustered has ln(4)>0
        let t_bare = make_torus("auth", 5, 0, 0);
        let t_clustered = make_torus("auth", 5, 0, 3);
        assert!(
            t_clustered.weight(0) > t_bare.weight(0),
            "clustered torus should outweigh bare: {} vs {}",
            t_clustered.weight(0), t_bare.weight(0)
        );
    }

    #[test]
    fn torus_weight_nonzero_when_has_cooccur() {
        // cluster_density = ln(cooccur+1). With 1 cooccur: ln(2) ≈ 0.693 > 0
        let t = make_torus("safe", 1, 0, 1);
        let w = t.weight(0);
        assert!(w > 0.0, "torus with 1 cooccur should have nonzero weight: {w}");
    }

    #[test]
    fn torus_weight_zero_when_no_cooccur() {
        // New tori have weight=0 until they co-occur — intentional (not retrieved until seeded)
        let t = make_torus("new", 5, 0, 0);
        assert_eq!(t.weight(0), 0.0);
    }

    // ── Foam insert + promote ─────────────────────────────────

    #[test]
    fn foam_insert_promote_increments_winding() {
        let mut foam = Foam::new();
        foam.insert("auth", 10);
        let w1 = foam.tori.values().next().unwrap().winding;
        foam.insert("auth", 10);
        let w2 = foam.tori.values().next().unwrap().winding;
        assert_eq!(w2, w1 + 1, "second insert should promote winding");
    }

    #[test]
    fn foam_insert_increments_count() {
        let mut foam = Foam::new();
        foam.insert("token", 5);
        foam.insert("token", 5);
        foam.insert("token", 5);
        let t = foam.tori.values().next().unwrap();
        assert_eq!(t.count, 3, "count should equal number of insert calls");
    }

    // ── Wire 2: co-occurrence ─────────────────────────────────

    #[test]
    fn foam_observe_cooccur_bidirectional() {
        let mut foam = Foam::new();
        foam.insert("auth", 5);
        foam.insert("db", 5);
        foam.observe_cooccur("auth", "db");

        let id_db: [u8; 32] = blake3::hash(b"db").into();
        let id_auth: [u8; 32] = blake3::hash(b"auth").into();

        let hex_auth: String = id_auth.iter().map(|b| format!("{:02x}", b)).collect();
        let hex_db: String = id_db.iter().map(|b| format!("{:02x}", b)).collect();

        let auth_cooccur = &foam.tori[&hex_auth].cooccur;
        let db_cooccur = &foam.tori[&hex_db].cooccur;

        assert!(auth_cooccur.contains(&id_db), "auth should record db as co-occur");
        assert!(db_cooccur.contains(&id_auth), "db should record auth as co-occur");
    }

    #[test]
    fn foam_observe_cooccur_dedup() {
        let mut foam = Foam::new();
        foam.insert("a", 5);
        foam.insert("b", 5);
        foam.observe_cooccur("a", "b");
        foam.observe_cooccur("a", "b");
        foam.observe_cooccur("a", "b");

        let id_a: [u8; 32] = blake3::hash(b"a").into();
        let hex_a: String = id_a.iter().map(|b| format!("{:02x}", b)).collect();
        let id_b: [u8; 32] = blake3::hash(b"b").into();
        let cooccur_count = foam.tori[&hex_a].cooccur.iter().filter(|&&x| x == id_b).count();
        assert_eq!(cooccur_count, 1, "duplicate cooccur should be deduplicated");
    }

    // ── Wire 3: agency frame ──────────────────────────────────

    #[test]
    fn foam_observe_frame_updates_bias() {
        let mut foam = Foam::new();
        foam.insert("auth", 5);
        foam.observe_frame("auth", AgencyFrame::Command);

        let id: [u8; 32] = blake3::hash(b"auth").into();
        let hex: String = id.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(foam.tori[&hex].agency_bias, AgencyFrame::Command);
    }

    #[test]
    fn foam_observe_frame_none_does_not_override() {
        let mut foam = Foam::new();
        foam.insert("db", 5);
        foam.observe_frame("db", AgencyFrame::Query);
        foam.observe_frame("db", AgencyFrame::None);

        let id: [u8; 32] = blake3::hash(b"db").into();
        let hex: String = id.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(foam.tori[&hex].agency_bias, AgencyFrame::Query, "None frame should not override existing bias");
    }

    // ── Wire 4: weight-driven BARF scoring ───────────────────

    #[test]
    fn barf_returns_highest_weight_torus() {
        let t_heavy = make_torus("heavy", 20, 0, 3);  // high weight
        let t_light = make_torus("light", 1, 0, 0);   // low weight

        let mut tori: HashMap<[u8; 32], Torus> = HashMap::new();
        tori.insert(t_heavy.id, t_heavy.clone());
        tori.insert(t_light.id, t_light.clone());

        let q = BarfQuery::new("heavy query").with_turn(0);
        let results = barf(&q, &tori);

        assert!(!results.is_empty(), "should return results");
        // heavy should score higher than light regardless of distance
        let heavy_score = results.iter().find(|r| r.torus.label == "heavy").map(|r| r.score).unwrap_or(0.0);
        let light_score = results.iter().find(|r| r.torus.label == "light").map(|r| r.score).unwrap_or(0.0);
        assert!(heavy_score > light_score, "heavy (count=20, cooccur=3) should outscore light (count=1)");
    }

    #[test]
    fn barf_truncates_to_max_results() {
        let mut tori: HashMap<[u8; 32], Torus> = HashMap::new();
        for i in 0..20 {
            let t = make_torus(&format!("torus_{i}"), 5, 0, 0);
            tori.insert(t.id, t);
        }
        let mut q = BarfQuery::new("test");
        q.max_results = 3;
        let results = barf(&q, &tori);
        assert!(results.len() <= 3, "should respect max_results");
    }

    // ── Serialization roundtrip ───────────────────────────────

    #[test]
    fn foam_roundtrip_json() {
        let mut foam = Foam::new();
        foam.insert("auth", 10);
        foam.insert("db", 5);
        foam.observe_cooccur("auth", "db");
        foam.observe_frame("auth", AgencyFrame::Query);
        foam.advance_turn();

        let json = serde_json::to_string(&foam).expect("serialize");
        let foam2: Foam = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(foam2.tori.len(), foam.tori.len());
        assert_eq!(foam2.turn, foam.turn);
    }

    // ── Turn tracking ─────────────────────────────────────────

    #[test]
    fn foam_advance_turn_increments() {
        let mut foam = Foam::new();
        assert_eq!(foam.turn, 0);
        foam.advance_turn();
        foam.advance_turn();
        assert_eq!(foam.turn, 2);
    }

    // ── XOR distance properties ───────────────────────────────

    #[test]
    fn xor_distance_self_is_zero() {
        let id: [u8; 32] = blake3::hash(b"test").into();
        assert_eq!(xor_distance(&id, &id), 0.0);
    }

    #[test]
    fn xor_distance_bounded() {
        let a: [u8; 32] = blake3::hash(b"alpha").into();
        let b: [u8; 32] = blake3::hash(b"beta").into();
        let d = xor_distance(&a, &b);
        assert!(d >= 0.0 && d <= 1.0, "distance must be in [0,1]: {d}");
    }
}
