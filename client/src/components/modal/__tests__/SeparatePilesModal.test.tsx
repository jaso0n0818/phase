import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { GameState, WaitingFor } from "../../../adapter/types.ts";
import { CardChoiceModal } from "../CardChoiceModal.tsx";
import { useGameStore } from "../../../stores/gameStore.ts";
import { useMultiplayerStore } from "../../../stores/multiplayerStore.ts";

const dispatchMock = vi.fn();

vi.mock("../../../hooks/useGameDispatch.ts", () => ({
  useGameDispatch: () => dispatchMock,
}));

function makeCreature(id: number, name: string, controller: number) {
  return {
    id,
    card_id: id,
    owner: controller,
    controller,
    zone: "Battlefield" as const,
    tapped: false,
    face_down: false,
    flipped: false,
    transformed: false,
    damage_marked: 0,
    dealt_deathtouch_damage: false,
    attached_to: null,
    attachments: [],
    counters: {},
    name,
    power: 1,
    toughness: 1,
    loyalty: null,
    card_types: { supertypes: [], core_types: ["Creature"], subtypes: [] },
    mana_cost: { type: "NoCost" as const },
    keywords: [],
    abilities: [],
    trigger_definitions: [],
    replacement_definitions: [],
    static_definitions: [],
    color: [],
    base_power: 1,
    base_toughness: 1,
    base_keywords: [],
    base_color: [],
    timestamp: 1,
    entered_battlefield_turn: 1,
  };
}

function baseState(waitingFor: WaitingFor): GameState {
  return {
    turn_number: 1,
    active_player: 0,
    phase: "Main1",
    players: [
      { id: 0, life: 20, poison_counters: 0, mana_pool: { mana: [] }, library: [], hand: [], graveyard: [], has_drawn_this_turn: false, lands_played_this_turn: 0, turns_taken: 0 },
      { id: 1, life: 20, poison_counters: 0, mana_pool: { mana: [] }, library: [], hand: [], graveyard: [], has_drawn_this_turn: false, lands_played_this_turn: 0, turns_taken: 0 },
    ],
    priority_player: 0,
    objects: {
      10: makeCreature(10, "Grizzly Bears", 1),
      11: makeCreature(11, "Llanowar Elves", 1),
      12: makeCreature(12, "Birds of Paradise", 1),
    },
    next_object_id: 100,
    battlefield: [10, 11, 12],
    stack: [],
    exile: [],
    rng_seed: 1,
    combat: null,
    waiting_for: waitingFor,
    has_pending_cast: false,
    lands_played_this_turn: 0,
    max_lands_per_turn: 1,
    priority_pass_count: 0,
    pending_replacement: null,
    layers_dirty: false,
    next_timestamp: 2,
    eliminated_players: [],
  } as unknown as GameState;
}

describe("SeparatePilesPartitionModal (via CardChoiceModal)", () => {
  beforeEach(() => {
    dispatchMock.mockClear();
    const waitingFor: WaitingFor = {
      type: "SeparatePilesPartition",
      data: {
        player: 1,
        eligible: [10, 11, 12],
        remaining_subjects: [],
        completed: [],
        chooser: 0,
        source_id: 99,
      },
    };
    const state = baseState(waitingFor);
    useMultiplayerStore.setState({ activePlayerId: 1 });
    useGameStore.setState({
      gameMode: "online",
      gameState: state,
      waitingFor,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("submits an empty pile A when nothing is toggled (CR 700.3d)", () => {
    render(<CardChoiceModal />);
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "SubmitPilePartition",
      data: { pile_a: [] },
    });
  });

  it("toggles cards into pile A in engine-provided order", () => {
    render(<CardChoiceModal />);
    // Toggle Llanowar Elves (id 11) and Birds of Paradise (id 12) into pile A;
    // Grizzly Bears (id 10) stays in pile B.
    fireEvent.click(screen.getByLabelText("Birds of Paradise — pile B"));
    fireEvent.click(screen.getByLabelText("Llanowar Elves — pile B"));
    fireEvent.click(screen.getByRole("button", { name: "Confirm" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "SubmitPilePartition",
      data: { pile_a: [11, 12] },
    });
  });
});

describe("SeparatePilesChoiceModal (via CardChoiceModal)", () => {
  beforeEach(() => {
    dispatchMock.mockClear();
    const waitingFor: WaitingFor = {
      type: "SeparatePilesChoice",
      data: {
        player: 0,
        pending: [],
        current: {
          subject: 1,
          pile_a: [10, 11],
          pile_b: [12],
        },
        source_id: 99,
      },
    };
    const state = baseState(waitingFor);
    useMultiplayerStore.setState({ activePlayerId: 0 });
    useGameStore.setState({
      gameMode: "online",
      gameState: state,
      waitingFor,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("dispatches ChoosePile A", () => {
    render(<CardChoiceModal />);
    fireEvent.click(screen.getByRole("button", { name: "Choose Pile A" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "ChoosePile",
      data: { pile: { type: "A" } },
    });
  });

  it("dispatches ChoosePile B", () => {
    render(<CardChoiceModal />);
    fireEvent.click(screen.getByRole("button", { name: "Choose Pile B" }));
    expect(dispatchMock).toHaveBeenCalledWith({
      type: "ChoosePile",
      data: { pile: { type: "B" } },
    });
  });
});
