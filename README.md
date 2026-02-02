# Feynman: The Stochastic Teacher

A Rust-based CLI tool and Claude skill for deep learning using the **Feynman Technique** and **spaced repetition**.

## Philosophy

The **Feynman Technique** is a learning method developed by physicist Richard Feynman:

1. **Choose a concept** to learn
2. **Teach it simply** as if explaining to a child
3. **Identify gaps** where your explanation breaks down
4. **Review and simplify** until you truly understand

Combined with **stochastic spaced repetition**, this tool helps you:
- Track topics you're learning
- Schedule reviews based on mastery level
- Identify and address knowledge gaps
- Build deep, lasting understanding

## Features

- **Topic Management**: Add, list, and organize topics with tags
- **Progress Tracking**: Mastery levels (0-5), success rates, review history
- **Stochastic Selection**: Weighted random selection favoring overdue/weak topics
- **Spaced Repetition**: Automatic scheduling based on performance
- **Terminal UI**: Interactive vim-style TUI for browsing topics and plans
- **Claude Integration**: AI-powered Feynman technique sessions
- **JSON Output**: For scripting and integration

## Installation

### Prerequisites

- Rust 1.70+ ([install](https://rustup.rs))

### Build & Install

```bash
git clone https://github.com/alexjperkins/feynman.git
cd feynman
make install
```

This builds an optimized release binary and installs it to `/usr/local/bin/feynman`.

### Initialize Database

```bash
feynman init
```

Database is stored in the platform config directory by default (see Environment Variables).

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `FEYNMAN_DB` | Path to SQLite database | Platform config dir (e.g., `~/.config/feynman/` on Linux) |
| `CLAUDE_SKILLS_CONFIG` | Directory for Claude skill installation | `~/.claude/commands` |

Example:
```bash
export FEYNMAN_DB=/path/to/your/feynman.db
export CLAUDE_SKILLS_CONFIG=~/my-claude-skills
```

## Usage

### Managing Topics

```bash
# Add a topic with tags
feynman topic add "Rust Ownership" \
  --description "Move semantics, borrowing, lifetimes" \
  --tags rust,memory,core-concepts

# List all topics
feynman topic list

# Filter by tag
feynman topic list --tag rust

# View topic details and progress
feynman topic show 1

# Update tags
feynman topic tag 1 --tags rust,memory,advanced

# Delete a topic
feynman topic delete 1
```

### Learning Sessions

```bash
# Get next topic to review (stochastic selection)
feynman next

# Filter by tag
feynman next --tag rust

# After review, record outcome
feynman review 1 --outcome success
feynman review 1 --outcome partial --notes "Struggled with lifetimes"
feynman review 1 --outcome fail --notes "Need to revisit basics"
```

### Progress Tracking

```bash
# Overall statistics
feynman stats

# All tags
feynman tags

# Detailed topic progress
feynman topic show 1
```

### JSON Output

All commands support `--json` for programmatic use:

```bash
feynman --json topic list
feynman --json next
feynman --json stats
```

### Terminal UI

Launch the interactive TUI to browse topics, plans, and progress:

```bash
feynman tui
```

#### Views

| View | Description |
|------|-------------|
| Dashboard | Stats, due topics, recent sessions |
| Topics | Browse all topics with mastery and skill levels |
| Topic Detail | Progress, gaps, session history |
| Plans | Browse interview plans |
| Plan Detail | Interview entries by category |

#### Keybindings (Vim-style)

| Key | Action |
|-----|--------|
| `h` / `l` | Navigate views (left/right) |
| `j` / `k` | Navigate list items (down/up) |
| `g` / `G` | Jump to top/bottom of list |
| `Enter` or `l` | Open detail view |
| `Esc` or `h` | Back / Clear filter |
| `/` | Filter topics by tag |
| `Ctrl+r` | Refresh data |
| `q` | Quit |

## Mastery Levels

| Level | Label       | Next Review |
|-------|-------------|-------------|
| 0     | New         | 1 day       |
| 1     | Learning    | 2 days      |
| 2     | Familiar    | 4 days      |
| 3     | Comfortable | 7 days      |
| 4     | Proficient  | 14 days     |
| 5     | Mastered    | 30 days     |

## Stochastic Selection

The `next` command doesn't just pick the most overdue topic. It uses weighted randomness:

- **Overdue topics** have higher weight
- **Lower mastery** topics have higher weight
- **Random factor** ensures variety

This prevents getting stuck reviewing the same topics and ensures comprehensive coverage.

## Claude Skill Integration

Install the Claude skill:

```bash
make install-skill
```

This installs to `$CLAUDE_SKILLS_CONFIG` (default: `~/.claude/commands`). Prompts before overwriting existing files.

### Three Modes

| Mode | Trigger | Who Leads | Purpose |
|------|---------|-----------|---------|
| **Feynman** | "check my understanding of X" | User explains | Identify knowledge gaps through teaching |
| **Socratic** | "teach me about X" | Claude asks questions | Guide discovery through questioning |
| **Plan** | "plan X" / "spec X" | Claude interviews | Extract requirements, produce design doc |

### Feynman Mode
1. User invokes `/feynman` or asks to check understanding
2. Claude asks user to explain the topic simply
3. Claude probes gaps and misconceptions
4. User re-explains until understanding is solid
5. Claude records the outcome

### Socratic Mode
1. User asks to learn about a topic
2. Claude assesses skill level (asks or infers)
3. Claude guides discovery through questions only (no lecturing)
4. Records outcome with gaps identified

### Plan Mode (Technical Interview)
1. User describes a fuzzy task ("plan X", "spec X")
2. Claude conducts a structured interview:
   - Problem & context
   - Scope & requirements
   - Technical design
   - Edge cases & failure modes
   - Security & operations
   - Definition of done
3. Claude generates a markdown spec at user-specified path

## Tag Taxonomy

Suggested consistent tagging:

| Category   | Examples                                    |
|------------|---------------------------------------------|
| Domain     | `rust`, `go`, `python`, `math`, `cs`        |
| Type       | `core-concepts`, `patterns`, `algorithms`   |
| Difficulty | `beginner`, `intermediate`, `advanced`      |
| Status     | `struggling`, `priority`, `favorite`        |

## Development

```bash
# Build (debug)
make build

# Run tests
make test

# Format + lint + test
make check

# Build release
make release
```

## Project Structure

```
feynman/
├── Cargo.toml              # Rust dependencies
├── Makefile                # Build automation
├── README.md               # This file
├── src/
│   ├── main.rs             # CLI entry point
│   ├── db.rs               # SQLite operations
│   ├── models.rs           # Data structures
│   └── tui/                # Terminal UI
│       ├── mod.rs          # App state, event loop
│       ├── ui.rs           # Layout and rendering
│       └── widgets/        # View components
│           ├── dashboard.rs
│           ├── topics.rs
│           ├── topic_detail.rs
│           ├── plans.rs
│           └── plan_detail.rs
└── .claude/
    └── commands/
        └── feynman.md      # Claude skill definition
```

## License

MIT
