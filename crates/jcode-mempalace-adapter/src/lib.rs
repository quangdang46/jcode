// =====================================================================
// jcode-mempalace-adapter — bridge jcode's MemoryEntry ↔ mempalace's Drawer
// =====================================================================
//
// This crate provides the **type-conversion layer** between jcode's
// `MemoryEntry` / `MemoryCategory` / `MemoryScope` and mempalace's
// `Drawer` / `DrawerKind` / `MemoryScope`.
//
// To avoid the `libsqlite3-sys` version conflict between mempalace
// (rusqlite 0.32) and jcode (rusqlite 0.33, via casr), this crate
// does NOT depend on `mempalace-core`.  Instead it defines local
// mirror types (`Drawer`, `DrawerKind`, `DrawerId`, `MemoryScope`)
// that match mempalace's public surface exactly.  When the full
// backend integration is ready (after rusqlite versions align),
// the mirrors will be replaced with `cfg(feature = "backend")`
// gates that pull in the real types.
//
// # What's here now (Issue #355)
//
// - `convert::category_to_kind` / `kind_to_category` — 1:1 mapping
//   between `MemoryCategory` and `DrawerKind`
// - `convert::memory_entry_to_drawer` / `drawer_to_memory_entry`
//   — full field-level round-trip conversion
// - `convert::mp_scope_from_jcode` / `jcode_scope_from_mp` — scope
//   conversion
// - Mirror types: `Drawer`, `DrawerKind`, `DrawerId`, `MemoryScope`
//   — exported for downstream crates that need to construct
//   mempalace-shaped values without pulling in the full core crate
//
// # What's deferred
//
// - Issue #356: Data migration tool (needs Palace runtime)
// - Issue #357: MemoryTool config gate (needs Palace runtime)
// - Issue #358: Prompt injection pipeline (needs Palace runtime)
// - Issue #359: Integration tests (needs Palace runtime)

pub mod convert;

// Re-export mirror types at crate root for ergonomic imports.
pub use convert::{
    Drawer, DrawerId, DrawerKind, MemoryScope, MpReinforcement, category_to_kind,
    drawer_to_memory_entry, jcode_scope_from_mp, kind_to_category, memory_entry_to_drawer,
    mp_scope_from_jcode, string_to_trust, trust_to_string,
};

// ---- tests ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::convert::*;
    use jcode_memory_types::{MemoryCategory, MemoryEntry, MemoryScope as JcodeScope, TrustLevel};

    fn test_entry(content: &str, category: MemoryCategory) -> MemoryEntry {
        MemoryEntry {
            id: "mem-test".to_string(),
            category,
            content: content.to_string(),
            tags: vec!["test".to_string()],
            search_text: content.to_lowercase(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            access_count: 0,
            source: Some("test".to_string()),
            trust: TrustLevel::Medium,
            strength: 1,
            active: true,
            superseded_by: None,
            reinforcements: vec![],
            embedding: None,
            confidence: 1.0,
        }
    }

    #[test]
    fn round_trip_conversion_preserves_content() {
        let original = test_entry("use Rust for memory", MemoryCategory::Fact);
        let drawer = memory_entry_to_drawer(&original, JcodeScope::Project);
        let back = drawer_to_memory_entry(&drawer);
        assert_eq!(back.content, original.content);
        assert_eq!(back.category, original.category);
        assert_eq!(back.tags, original.tags);
        assert_eq!(back.confidence, original.confidence);
        assert_eq!(back.active, original.active);
    }

    #[test]
    fn category_to_drawer_kind_maps_correctly() {
        assert_eq!(category_to_kind(&MemoryCategory::Fact), DrawerKind::Fact);
        assert_eq!(
            category_to_kind(&MemoryCategory::Preference),
            DrawerKind::Preference
        );
        assert_eq!(
            category_to_kind(&MemoryCategory::Entity),
            DrawerKind::Entity
        );
        assert_eq!(
            category_to_kind(&MemoryCategory::Correction),
            DrawerKind::Correction
        );
        assert_eq!(
            category_to_kind(&MemoryCategory::Custom("snippet".into())),
            DrawerKind::Custom("snippet".into())
        );
    }

    #[test]
    fn kind_to_category_maps_correctly() {
        assert_eq!(kind_to_category(&DrawerKind::Fact), MemoryCategory::Fact);
        assert_eq!(
            kind_to_category(&DrawerKind::Preference),
            MemoryCategory::Preference
        );
        assert_eq!(
            kind_to_category(&DrawerKind::Entity),
            MemoryCategory::Entity
        );
        assert_eq!(
            kind_to_category(&DrawerKind::Correction),
            MemoryCategory::Correction
        );
        assert_eq!(
            kind_to_category(&DrawerKind::Custom("ref".into())),
            MemoryCategory::Custom("ref".into())
        );
        // Non-jcode kinds map to Fact as a safe default.
        assert_eq!(kind_to_category(&DrawerKind::Event), MemoryCategory::Fact);
        assert_eq!(
            kind_to_category(&DrawerKind::Discovery),
            MemoryCategory::Fact
        );
        assert_eq!(kind_to_category(&DrawerKind::Advice), MemoryCategory::Fact);
        assert_eq!(kind_to_category(&DrawerKind::Raw), MemoryCategory::Fact);
    }

    #[test]
    fn scope_conversion_round_trips() {
        let pairs = [
            (JcodeScope::Project, MemoryScope::Local),
            (JcodeScope::Global, MemoryScope::Global),
            (JcodeScope::All, MemoryScope::All),
        ];
        for (jcode, mp) in &pairs {
            assert_eq!(mp_scope_from_jcode(jcode.clone()), mp.clone());
            assert_eq!(jcode_scope_from_mp(mp), jcode.clone());
        }
    }

    #[test]
    fn drawer_builder_sets_defaults() {
        let d = Drawer::new("hello");
        assert_eq!(d.content, "hello");
        assert_eq!(d.kind, DrawerKind::Raw);
        assert!(d.active);
        assert!((d.confidence - 1.0).abs() < 0.01);
        assert_eq!(d.consolidation_strength, 1);
        assert!(d.tags.is_empty());
    }

    #[test]
    fn half_life_days_matches_jcode() {
        assert!((DrawerKind::Correction.half_life_days() - 365.0).abs() < 0.01);
        assert!((DrawerKind::Preference.half_life_days() - 90.0).abs() < 0.01);
        assert!((DrawerKind::Entity.half_life_days() - 60.0).abs() < 0.01);
        assert!((DrawerKind::Fact.half_life_days() - 30.0).abs() < 0.01);
        assert!((DrawerKind::Custom("x".into()).half_life_days() - 45.0).abs() < 0.01);
    }
}
