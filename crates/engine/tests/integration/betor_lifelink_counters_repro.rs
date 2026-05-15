//! Issue #321 regression: Betor, Ancestor's Voice end-step trigger applied
//! double the +1/+1 counters.
//!
//! Oracle: "Flying, lifelink. At the beginning of your end step, put a number
//! of +1/+1 counters on up to one other target creature you control equal to
//! the amount of life you gained this turn. Return up to one target creature
//! card with mana value less than or equal to the amount of life you lost this
//! turn from your graveyard to the battlefield."
//!
//! Root cause: the end-step trigger is a two-effect chain — `PutCounter`
//! (slot 0) followed by a `ChangeZone` sub-ability (slot 1), both `up to one`.
//! `assign_selected_slots_recursive` computed how many target slots the
//! `PutCounter` node should consume as `total_slots - sub_chain_minimum`.
//! Because the `ChangeZone` sub-ability is itself `up to one` (minimum 0), the
//! `PutCounter` node greedily claimed BOTH chosen targets. `resolve_add` then
//! iterated `ability.targets` and applied the counters once per entry —
//! doubling the counters whenever both trigger slots were filled.
//!
//! The fix caps each multi-target node at its own resolved `multi_target` max
//! (mirroring the per-node slot count `collect_target_slots` produces), so each
//! effect resolves against exactly its own chosen targets (CR 601.2c).
//!
//! CR references (verified against docs/MagicCompRules.txt):
//!   - CR 120.3f: "Damage dealt by a source with lifelink causes that source's
//!     controller to gain that much life, in addition to the damage's other
//!     results." Lifelink is NOT a separate triggered ability — the life gain
//!     is simultaneous with the damage, incrementing `life_gained_this_turn`
//!     exactly once.
//!   - CR 513.1a: "At the beginning of [your] end step" triggers fire when the
//!     end step begins.
//!   - CR 601.2c: each effect in a chained ability is assigned its own targets.

use super::rules::{run_combat, GameScenario, Phase, WaitingFor, P0, P1};
use engine::types::ability::TargetRef;
use engine::types::actions::GameAction;
use engine::types::counter::CounterType;
use engine::types::identifiers::ObjectId;

const BETOR_ORACLE: &str = "Flying, lifelink\nAt the beginning of your end step, put a number of +1/+1 counters on up to one other target creature you control equal to the amount of life you gained this turn. Return up to one target creature card with mana value less than or equal to the amount of life you lost this turn from your graveyard to the battlefield.";

/// Drive the engine from the post-combat priority window to the end-step
/// trigger's target-selection window. Returns once the trigger is asking for
/// its first target.
fn advance_to_end_step_trigger(runner: &mut super::rules::GameRunner) {
    for _ in 0..60 {
        match runner.state().waiting_for.clone() {
            WaitingFor::TriggerTargetSelection { .. } | WaitingFor::TargetSelection { .. } => {
                return;
            }
            WaitingFor::Priority { .. } => {
                if runner.act(GameAction::PassPriority).is_err() {
                    return;
                }
            }
            other => panic!("unexpected waiting state before end-step trigger: {other:?}"),
        }
    }
    panic!("phase machine did not reach the end-step trigger");
}

/// Count `+1/+1` counters on an object.
fn p1p1_counters(runner: &super::rules::GameRunner, id: ObjectId) -> u32 {
    runner
        .state()
        .objects
        .get(&id)
        .expect("object still present")
        .counters
        .get(&CounterType::Plus1Plus1)
        .copied()
        .unwrap_or(0)
}

/// Bare repro with the second (graveyard-return) slot declined: Betor (3/3
/// flying lifelink) attacks unblocked for 3, gaining 3 life. At end step the
/// trigger puts counters on the chosen receiver. With no Doubling Season /
/// Hardened Scales anywhere, the receiver must get exactly 3 counters.
#[test]
fn betor_end_step_counters_equal_lifelink_gain_no_doubler() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // Betor built from real Oracle text → trigger AST produced by the parser.
    let betor = scenario
        .add_creature_from_oracle(P0, "Betor, Ancestor's Voice", 3, 3, BETOR_ORACLE)
        .id();
    let receiver = scenario.add_creature(P0, "Receiver", 1, 1).id();
    scenario.add_creature(P1, "Blocker", 3, 3);

    let mut runner = scenario.build();
    let life_before = runner.life(P0);

    // Betor attacks P1 unblocked → 3 combat damage → 3 lifelink life.
    run_combat(&mut runner, vec![betor], vec![]);

    assert_eq!(
        runner.life(P0),
        life_before + 3,
        "Betor lifelink must gain exactly 3 life from 3 combat damage (CR 120.3f)"
    );
    assert_eq!(
        runner.state().players[0].life_gained_this_turn,
        3,
        "life_gained_this_turn must be exactly 3 — one lifelink combat-damage \
         event increments it once (CR 120.3f)"
    );

    advance_to_end_step_trigger(&mut runner);

    // Slot 0: PutCounter receiver. Slot 1: graveyard return — decline (no
    // creature cards in the graveyard, the realistic case).
    let mut guard = 0;
    while matches!(
        runner.state().waiting_for,
        WaitingFor::TriggerTargetSelection { .. } | WaitingFor::TargetSelection { .. }
    ) {
        guard += 1;
        assert!(guard < 10, "target selection did not terminate");

        let target = match &runner.state().waiting_for {
            WaitingFor::TriggerTargetSelection { selection, .. }
            | WaitingFor::TargetSelection { selection, .. } => {
                if selection.current_slot == 0 {
                    Some(TargetRef::Object(receiver))
                } else {
                    None
                }
            }
            _ => None,
        };
        runner
            .act(GameAction::ChooseTarget { target })
            .expect("ChooseTarget should succeed");
    }

    runner.advance_until_stack_empty();

    assert_eq!(
        p1p1_counters(&runner, receiver),
        3,
        "Betor end-step trigger must put exactly 3 +1/+1 counters on the \
         receiver (= 3 life gained via lifelink), not double"
    );
}

/// Issue #321 core regression: when BOTH end-step trigger slots are filled,
/// the `PutCounter` effect must apply ONLY to its own slot-0 target. Before the
/// fix, the `PutCounter` node greedily consumed the slot-1 target as well, so
/// the counters were applied to both creatures (and the receiver received
/// double).
///
/// Slot 1 (the `ChangeZone` graveyard-return) surfaces battlefield creatures as
/// legal choices in this harness; that secondary targeting quirk is orthogonal
/// to this test. The point under test is slot ATTRIBUTION: whatever slot 1
/// holds, the slot-0 `PutCounter` effect must not touch it.
#[test]
fn betor_put_counter_does_not_leak_into_second_trigger_slot() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    let betor = scenario
        .add_creature_from_oracle(P0, "Betor, Ancestor's Voice", 3, 3, BETOR_ORACLE)
        .id();
    let receiver = scenario.add_creature(P0, "Receiver", 1, 1).id();
    // A second P0 creature that will be selected for slot 1.
    let slot_one_pick = scenario.add_creature(P0, "Slot One Pick", 2, 2).id();
    scenario.add_creature(P1, "Blocker", 3, 3);

    let mut runner = scenario.build();
    run_combat(&mut runner, vec![betor], vec![]);
    assert_eq!(runner.state().players[0].life_gained_this_turn, 3);

    advance_to_end_step_trigger(&mut runner);

    // Slot 0 → receiver, slot 1 → a different creature. The fix guarantees the
    // PutCounter effect consumes only slot 0.
    let mut guard = 0;
    while matches!(
        runner.state().waiting_for,
        WaitingFor::TriggerTargetSelection { .. } | WaitingFor::TargetSelection { .. }
    ) {
        guard += 1;
        assert!(guard < 10, "target selection did not terminate");

        let target = match &runner.state().waiting_for {
            WaitingFor::TriggerTargetSelection {
                target_slots,
                selection,
                ..
            }
            | WaitingFor::TargetSelection {
                target_slots,
                selection,
                ..
            } => {
                let want = if selection.current_slot == 0 {
                    receiver
                } else {
                    slot_one_pick
                };
                // Only choose the intended object when it is a legal target
                // for this slot; otherwise decline so the test stays robust.
                target_slots.get(selection.current_slot).and_then(|slot| {
                    slot.legal_targets
                        .iter()
                        .find(|t| matches!(t, TargetRef::Object(id) if *id == want))
                        .cloned()
                })
            }
            _ => None,
        };
        runner
            .act(GameAction::ChooseTarget { target })
            .expect("ChooseTarget should succeed");
    }

    runner.advance_until_stack_empty();

    // The PutCounter slot-0 target receives exactly 3 counters.
    assert_eq!(
        p1p1_counters(&runner, receiver),
        3,
        "slot-0 PutCounter target must get exactly 3 +1/+1 counters"
    );
    // The slot-1 target must receive ZERO counters — the PutCounter effect
    // must not leak into the graveyard-return slot.
    assert_eq!(
        p1p1_counters(&runner, slot_one_pick),
        0,
        "slot-1 target must NOT receive +1/+1 counters — the PutCounter effect \
         must resolve against only its own chosen target (CR 601.2c)"
    );
}
