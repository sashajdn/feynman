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

Database is stored at `~/.config/feynman/feynman.db` by default.

Override with:
```bash
export FEYNMAN_DB=/path/to/your/feynman.db
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

The Claude skill (`.claude/commands/feynman.md`) enables AI-powered Feynman technique sessions:

1. User invokes `/feynman` or asks to learn/review
2. Claude retrieves next topic via CLI
3. Claude conducts a Feynman teaching session:
   - Asks user to explain the topic simply
   - Identifies gaps in understanding
   - Provides targeted explanations
   - Has user re-explain
4. Claude records the outcome

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
│   └── models.rs           # Data structures
└── .claude/
    └── commands/
        └── feynman.md      # Claude skill definition
```

## License

MIT
