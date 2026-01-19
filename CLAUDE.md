# Esport Signal - Project Configuration

## Project Goal
Build a live Dota 2 data aggregation system that generates betting signals for Polymarket esports markets.

---

## Technical Decisions

### Language/Runtime
Rust

### API Strategy (Free Tier Start)
| Purpose | API | Priority |
|---------|-----|----------|
| Live match data | STRATZ GraphQL | Primary |
| Live match backup | Steam Web API | Fallback |
| Historical stats | OpenDota | Enrichment |
| Market data | Polymarket Gamma | Required |
| Schedules | Liquipedia scrapers | Optional |

### Data Storage
SQLite

---

## Architecture Guidelines

### Polling Intervals
```
POLYMARKET_SCAN_INTERVAL=300      # 5 min - check for new markets
LIVE_MATCH_POLL_INTERVAL=5        # 5 sec - during active matches
HISTORICAL_REFRESH_INTERVAL=3600  # 1 hour - background enrichment
```

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
│   └── team_aliases.json     # Team name mapping
└── tests/
```

---

## Development Rules

### Do
- Always check Polymarket for active markets before fetching live data
- Implement exponential backoff for API failures
- Log all signals with timestamps for backtesting
- Use team alias mapping for name resolution
- Cache historical data to reduce API calls

### Don't
- Don't poll APIs for matches without active Polymarket markets
- Don't hardcode API keys (use environment variables)
- Don't auto-execute trades in MVP (log only)
- Don't exceed free tier rate limits

---

## Notes & Learnings
<!-- Add insights as you build -->

---

## Links & Resources
- [Polymarket Dota 2](https://polymarket.com/sports/dota2/games)
- [STRATZ API](https://stratz.com/api)
- [STRATZ Live Matches](https://stratz.com/matches/live)
- [OpenDota API Docs](https://docs.opendota.com/)

---

*Last updated: 2026-01-20*
