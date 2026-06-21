//! Issue #3999 — Latchkey Faerie's ETB draw is gated on "if its prowl cost was
//! paid", but the intervening-if was dropped: it drew unconditionally.
//!
//! Oracle: "Flying\nProwl {2}{U}\nWhen this creature enters, if its prowl cost
//! was paid, draw a card."
//!
//! CR 702.76a + CR 603.4: "prowl cost was paid" is a cast-variant provenance
//! tag (`CastVariantPaid::Prowl`) recorded at resolution. The ETB intervening-if
//! must draw only when the permanent was cast for its prowl cost.

use engine::game::scenario::{GameScenario, P0};
use engine::game::zones::move_to_zone;
use engine::parser::oracle::parse_oracle_text;
use engine::types::ability::{CastVariantPaid, TriggerCondition};
use engine::types::phase::Phase;
use engine::types::zones::Zone;

const LATCHKEY: &str =
    "Flying\nProwl {2}{U}\nWhen this creature enters, if its prowl cost was paid, draw a card.";

/// Enter Latchkey from hand (firing its ETB), optionally tagged as a prowl cast,
/// and return how many cards the controller drew (library shrinkage).
fn draws_on_enter(prowl_paid: bool) -> usize {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let latchkey = scenario
        .add_creature_to_hand_from_oracle(P0, "Latchkey Faerie", 2, 2, LATCHKEY)
        .id();
    scenario.add_card_to_library_top(P0, "Island");
    scenario.add_card_to_library_top(P0, "Mountain");

    let mut runner = scenario.build();
    runner.state_mut().active_player = P0;
    let library_before = runner.state().players[P0.0 as usize].library.len();

    // Move Latchkey to the battlefield (generates the ETB ZoneChanged event).
    let mut events = Vec::new();
    move_to_zone(runner.state_mut(), latchkey, Zone::Battlefield, &mut events);

    // Tag the cast provenance after entry (battlefield-entry reset clears it),
    // mirroring the post-resolution tagging the stack does for a real prowl cast.
    if prowl_paid {
        let turn = runner.state().turn_number;
        runner
            .state_mut()
            .objects
            .get_mut(&latchkey)
            .unwrap()
            .cast_variant_paid = Some((CastVariantPaid::Prowl, turn));
    }

    engine::game::triggers::process_triggers(runner.state_mut(), &events);
    runner.advance_until_stack_empty();

    library_before.saturating_sub(runner.state().players[P0.0 as usize].library.len())
}

#[test]
fn latchkey_etb_is_gated_on_prowl_cost_paid() {
    // CR 702.76a: the ETB intervening-if must lower to CastVariantPaid { Prowl },
    // not be dropped (which left the draw unconditional — the reported bug).
    let parsed = parse_oracle_text(
        LATCHKEY,
        "Latchkey Faerie",
        &[],
        &["Creature".to_string()],
        &[],
    );
    let trigger = parsed
        .triggers
        .iter()
        .find(|t| t.condition.is_some())
        .expect("ETB trigger must carry an intervening-if condition");
    assert_eq!(
        trigger.condition,
        Some(TriggerCondition::CastVariantPaid {
            variant: CastVariantPaid::Prowl
        }),
        "\"if its prowl cost was paid\" must gate the ETB, got {:?}",
        trigger.condition
    );
}

#[test]
fn latchkey_draws_when_prowl_cost_was_paid() {
    assert_eq!(
        draws_on_enter(true),
        1,
        "Latchkey must draw a card when its prowl cost was paid"
    );
}

#[test]
fn latchkey_does_not_draw_when_hard_cast() {
    // The reported bug: it drew irrespective of whether prowl was paid.
    assert_eq!(
        draws_on_enter(false),
        0,
        "Latchkey must NOT draw when it was not cast for its prowl cost"
    );
}
