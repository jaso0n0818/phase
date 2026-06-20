//! Cloudshredder Sliver — "Sliver creatures you control have flying and haste."
//!
//! Regression coverage for the continuous static keyword-grant building block
//! (Layer 6 ability-adding effect, CR 613.1f) where ONE Oracle clause grants
//! TWO conjoined keywords — flying (CR 702.9) and haste (CR 702.10) — on the
//! subtype filter axis (Slivers). Axes: conjunction (both keywords from one
//! clause), subtype filter, self-inclusion, the "you control" exclusion, and
//! grant lifetime (CR 611.3).
//!
//! Drives the REAL parse → synthesis → layer pipeline and reads back the
//! EFFECTIVE post-`evaluate_layers` keyword set — a runtime test, not an
//! AST-shape test.

use engine::game::keywords::has_keyword;
use engine::game::layers::evaluate_layers;
use engine::game::scenario::{GameRunner, GameScenario, P0, P1};
use engine::types::identifiers::ObjectId;
use engine::types::keywords::Keyword;
use engine::types::phase::Phase;

const CLOUDSHREDDER_SLIVER: &str = "Sliver creatures you control have flying and haste.";

fn has_kw(runner: &mut GameRunner, id: ObjectId, keyword: &Keyword) -> bool {
    runner.state_mut().layers_dirty.mark_full();
    evaluate_layers(runner.state_mut());
    has_keyword(&runner.state().objects[&id], keyword)
}

#[test]
fn cloudshredder_sliver_grants_both_flying_and_haste_to_your_slivers() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    // Source: a Sliver carrying the conjoined grant (real parse + synthesis
    // pipeline). It is itself a Sliver you control.
    let cloud = scenario
        .add_creature_from_oracle(P0, "Cloudshredder Sliver", 1, 1, CLOUDSHREDDER_SLIVER)
        .with_subtypes(vec!["Sliver"])
        .id();

    // Another Sliver you control — gains BOTH keywords.
    let ally_sliver = scenario
        .add_creature(P0, "Muscle Sliver", 1, 1)
        .with_subtypes(vec!["Sliver"])
        .id();

    // A non-Sliver you control — outside the subtype filter.
    let ally_bear = scenario
        .add_creature(P0, "Grizzly Bears", 2, 2)
        .with_subtypes(vec!["Bear"])
        .id();

    // An opponent's Sliver — outside the "you control" filter.
    let foe_sliver = scenario
        .add_creature(P1, "Plated Sliver", 1, 1)
        .with_subtypes(vec!["Sliver"])
        .id();

    let mut runner = scenario.build();

    // CR 613.1f: Slivers you control (including the source) gain both keywords.
    assert!(
        has_kw(&mut runner, cloud, &Keyword::Flying) && has_kw(&mut runner, cloud, &Keyword::Haste),
        "Cloudshredder Sliver is a Sliver you control and must have flying AND haste"
    );
    assert!(
        has_kw(&mut runner, ally_sliver, &Keyword::Flying),
        "another Sliver you control gains flying"
    );
    assert!(
        has_kw(&mut runner, ally_sliver, &Keyword::Haste),
        "the SAME Sliver also gains haste (conjoined grant)"
    );

    // CR 205.3m: a non-Sliver you control is outside the subtype filter.
    assert!(
        !has_kw(&mut runner, ally_bear, &Keyword::Flying)
            && !has_kw(&mut runner, ally_bear, &Keyword::Haste),
        "a non-Sliver you control must gain neither keyword"
    );

    // CR 109.4: "you control" excludes the opponent's Sliver.
    assert!(
        !has_kw(&mut runner, foe_sliver, &Keyword::Flying)
            && !has_kw(&mut runner, foe_sliver, &Keyword::Haste),
        "an opponent's Sliver must gain neither keyword ('you control')"
    );
}

#[test]
fn cloudshredder_sliver_both_grants_turn_off_when_source_leaves() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);

    let cloud = scenario
        .add_creature_from_oracle(P0, "Cloudshredder Sliver", 1, 1, CLOUDSHREDDER_SLIVER)
        .with_subtypes(vec!["Sliver"])
        .id();
    let ally_sliver = scenario
        .add_creature(P0, "Muscle Sliver", 1, 1)
        .with_subtypes(vec!["Sliver"])
        .id();

    let mut runner = scenario.build();
    assert!(
        has_kw(&mut runner, ally_sliver, &Keyword::Flying)
            && has_kw(&mut runner, ally_sliver, &Keyword::Haste),
        "baseline: ally Sliver has both keywords while the source is present"
    );

    // CR 611.3: both continuous effects end when the source leaves.
    {
        let state = runner.state_mut();
        state.battlefield.retain(|&id| id != cloud);
        state.objects.remove(&cloud);
    }
    assert!(
        !has_kw(&mut runner, ally_sliver, &Keyword::Flying)
            && !has_kw(&mut runner, ally_sliver, &Keyword::Haste),
        "ally Sliver must lose BOTH keywords once the source is gone"
    );
}
