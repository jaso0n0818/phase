//! Nix — "Counter target spell if no mana was spent to cast it."
//!
//! Parser regression: the trailing intervening-if must lower to a
//! `QuantityCheck` over `ManaSpentToCast { scope: AbilityTarget }`, not
//! remain as swallowed `Condition_If` text.

use engine::parser::oracle::parse_oracle_text;
use engine::types::ability::{
    AbilityCondition, CastManaObjectScope, CastManaSpentMetric, Comparator, Effect, QuantityExpr,
    QuantityRef,
};

const NIX_ORACLE: &str = "Counter target spell if no mana was spent to cast it.";

#[test]
fn nix_parses_counter_with_target_no_mana_spent_condition() {
    let parsed = parse_oracle_text(NIX_ORACLE, "Nix", &[], &["Instant".to_string()], &[]);
    let ability = parsed
        .abilities
        .first()
        .expect("Nix must parse a spell ability");
    assert!(matches!(ability.effect.as_ref(), Effect::Counter { .. }));
    assert!(matches!(
        ability.condition.as_ref(),
        Some(AbilityCondition::QuantityCheck {
            lhs: QuantityExpr::Ref {
                qty: QuantityRef::ManaSpentToCast {
                    scope: CastManaObjectScope::AbilityTarget,
                    metric: CastManaSpentMetric::Total,
                },
            },
            comparator: Comparator::EQ,
            rhs: QuantityExpr::Fixed { value: 0 },
        })
    ));
}
