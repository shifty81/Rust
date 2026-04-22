# Quests (`*.quest.ron`) — **schema stub, not yet implemented**

Quest definitions currently hardcoded as the `QuestLog::DEFAULT_QUESTS`
array in `crates/atlas_voxel_planet/src/npc.rs` (5 fetch quests).

## Intended schema

```ron
(
    id: "collect_gravel",
    title: "Loose Footing",
    description: "The path is washing out.  Bring me 8 gravel blocks.",

    // Which NPC kind offers this quest; the existing E-key dialogue
    // chooses the first available quest whose prerequisites are met.
    offered_by: "TownElder",

    // Fetch-style objective.  Later: multi-step objective DAGs.
    objective: Fetch((voxel: "Gravel", count: 8)),

    // Rewards granted on turn-in.
    rewards: [
        GiveVoxel((voxel: "Stone", count: 16)),
    ],

    // Required prerequisites (other quest IDs already completed).
    prerequisites: [],
)
```
