# Database Schema

## Location
`data/signals.db` (SQLite)

---

## Table: signals

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Primary key, auto-increment |
| `market_condition_id` | TEXT | Polymarket condition ID (e.g., `0xa634...`) |
| `match_id` | INTEGER | OpenDota match ID |
| `market_team_a_odds` | REAL | Current market odds for team A (0.0-1.0) |
| `match_snapshot` | TEXT | JSON of `LiveMatchState` (see below) |
| `created_at` | TEXT | ISO 8601 timestamp |

### Indexes
- `idx_signals_market` on `market_condition_id`
- `idx_signals_match` on `match_id`
- `idx_signals_created` on `created_at`

---

## match_snapshot JSON Structure

```json
{
  "match_id": 8656602785,
  "league_name": "ESL Pro League",
  "radiant": {
    "name": "HEROIC",
    "team_id": 123456,
    "kills": 15,
    "towers_killed": 3,
    "barracks_killed": 0
  },
  "dire": {
    "name": "NEW GROWTH",
    "team_id": 789012,
    "kills": 8,
    "towers_killed": 1,
    "barracks_killed": 0
  },
  "gold_lead": 12500,
  "game_time": 1845,
  "is_live": true,
  "updated_at": "2026-01-20T05:12:01Z"
}
```

---

## Features for ML Model

Extract from `match_snapshot`:

| Feature | JSON Path | Description |
|---------|-----------|-------------|
| `gold_lead` | `.gold_lead` | Radiant gold advantage (negative = Dire leads) |
| `radiant_kills` | `.radiant.kills` | Radiant kill count |
| `dire_kills` | `.dire.kills` | Dire kill count |
| `radiant_towers` | `.radiant.towers_killed` | Towers destroyed by Radiant |
| `dire_towers` | `.dire.towers_killed` | Towers destroyed by Dire |
| `radiant_barracks` | `.radiant.barracks_killed` | Barracks destroyed by Radiant |
| `dire_barracks` | `.dire.barracks_killed` | Barracks destroyed by Dire |
| `game_time` | `.game_time` | Game duration in seconds |

**Target variable**: Match outcome (win/loss) - requires joining with match result after game ends.

---

**Note**: All fields are real data from OpenDota API.
