# Esport Signal - Project Configuration

## Project Goal
Build a live Dota 2 data aggregation system that generates betting signals for Polymarket esports markets.

---

## Technical Decisions

### Language/Runtime
Rust (tokio async runtime)

### API Strategy
| Purpose | API | Endpoint |
|---------|-----|----------|
| Market data | Polymarket Gamma | `/series/10309` (Dota 2 series) |
| Live match data | STRATZ GraphQL | `api.stratz.com/graphql` |
| Historical stats | OpenDota | `api.opendota.com/api` (future) |

### Data Storage
SQLite (`data/signals.db`)

---

## Architecture

### Workers (async tokio tasks)
1. **Market Scanner** - Polls Polymarket every 5 min for active Dota 2 markets
2. **Live Fetcher** - Polls STRATZ every 5 sec for live match data (only when markets exist)
3. **Signal Processor** - Generates signals from match updates, logs to SQLite

### Directory Structure
```
esport-signal/
├── Cargo.toml                # Dependencies
├── CLAUDE.md                 # This file
├── src/
│   ├── main.rs               # Entry point, worker spawning
│   ├── config.rs             # Environment config
│   ├── api/                  # STRATZ, Polymarket, OpenDota clients
│   ├── workers/              # Market scanner, live fetcher, signal processor
│   ├── models/               # Data types (market, match, signal)
│   ├── matching/             # Team name → match ID resolver
│   └── db/                   # SQLite signal logging
├── data/
│   ├── team_aliases.json     # Team name mapping
│   └── signals.db            # SQLite database (created on run)
└── tests/
```

### Environment Variables
```bash
STRATZ_API_TOKEN=xxx          # Required - get from stratz.com/api
POLYMARKET_API_URL=https://gamma-api.polymarket.com
DATABASE_URL=sqlite:data/signals.db
POLYMARKET_SCAN_INTERVAL=300  # 5 min
LIVE_MATCH_POLL_INTERVAL=5    # 5 sec
RUST_LOG=esport_signal=info
```

---

## Development Rules

### Do
- Always check Polymarket for active markets before fetching live data
- Implement exponential backoff for API failures
- Log all signals with timestamps for backtesting
- Use team alias mapping for name resolution

### Don't
- Don't poll APIs for matches without active Polymarket markets
- Don't hardcode API keys (use environment variables)
- Don't auto-execute trades in MVP (log only)
- Don't exceed free tier rate limits

---

## Notes & Learnings

### Polymarket API Structure
- Sports markets are under `/series/{id}` endpoint, not regular `/markets`
- Dota 2 series ID: `10309`
- Series endpoint returns events list (without markets)
- Must fetch `/events/{id}` individually to get markets array
- Market types: `moneyline` (match winner), `child_moneyline` (game winner), `kill_handicap`, etc.
- Fields use camelCase, `outcomes` and `outcomePrices` are JSON strings

### STRATZ API
- GraphQL endpoint requires Bearer token auth
- Live matches available via `live { matches { ... } }` query
- Building state is a bitmask (11 towers, 6 barracks per side)

---

## Links & Resources
- [Polymarket Dota 2](https://polymarket.com/sports/dota-2/games)
- [STRATZ API](https://stratz.com/api)
- [STRATZ Live Matches](https://stratz.com/matches/live)
- [OpenDota API Docs](https://docs.opendota.com/)

---

*Last updated: 2026-01-20*
