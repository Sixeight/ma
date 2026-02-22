# ma

A CLI tool that renders [Mermaid](https://mermaid.js.org/) diagrams as ASCII art. No browser, no image generation — just text.

## Install

```
cargo install --path .
```

## Usage

```
ma [OPTIONS] [FILE]
```

Reads from stdin if no file is given.

```bash
echo 'graph LR
    A --> B --> C' | ma
```

```
┌───┐     ┌───┐     ┌───┐
│ A │────>│ B │────>│ C │
└───┘     └───┘     └───┘
```

### Options

| Flag | Description |
|------|-------------|
| `-w, --width <N>` | Maximum output width in columns |

## Supported Diagrams

### Sequence Diagram

```bash
echo 'sequenceDiagram
    Alice->>Bob: Hello
    Bob-->>Alice: Hi there' | ma
```

```
┌───────┐    ┌─────┐
│ Alice │    │ Bob │
└───┬───┘    └──┬──┘
    │ Hello     │
    │──────────>│
    │           │
    │ Hi there  │
    │< ─ ─ ─ ─ ─│
    │           │
┌───┴───┐    ┌──┴──┐
│ Alice │    │ Bob │
└───────┘    └─────┘
```

Features:
- Arrow types: solid (`->>`, `->`), dotted (`-->>`, `-->`), cross (`-x`, `--x`)
- Participant aliases (`participant A as Alice`)
- Activation / deactivation (`activate`, `deactivate`, `+` / `-` shorthand)
- Self-messages (rendered as loops)
- Notes (`note right of`, `note left of`, `note over`)
- Blocks: `loop`, `alt`/`else`, `opt`, `break`, `par`/`and`, `critical`/`option`, `rect`
- Create / destroy participants
- Auto-numbering (`autonumber`)

### Flowchart (Graph)

Supports both `graph` and `flowchart` keywords with TD (top-down) and LR (left-right) directions.

```bash
echo 'graph TD
    A{Decision} -->|Yes| B[Action]
    A -->|No| C(Skip)' | ma
```

```
    ╱──────────╲
    │ Decision │
    ╲─────┬────╱
    ┌─────┴─────┐
    ▼           ▼
┌────────┐   ╭──────╮
│ Action │   │ Skip │
└────────┘   ╰──────╯
```

Features:
- Directions: TD/TB (top-down), LR (left-right)
- Node shapes: rectangle `[]`, round `()`, diamond `{}`, circle `(())`
- Edge types: arrow `-->`, open `---`, dotted `-.->`, thick `==>` (and link variants)
- Edge labels (`-->|label|` or `-- label -->`)
- Fan-out / fan-in with L-shaped edge routing
- Subgraphs (`subgraph`...`end`)
- Multi-target edges (`A --> B & C`)

### ER Diagram

```bash
echo 'erDiagram
    CUSTOMER ||--o{ ORDER : places
    ORDER ||--|{ LINE_ITEM : contains' | ma
```

```
┌──────────┐              ┌───────┐                ┌───────────┐
│ CUSTOMER │||──places──o{│ ORDER │||──contains──|{│ LINE_ITEM │
└──────────┘              └───────┘                └───────────┘
```

Features:
- Cardinality symbols: `||` (exactly one), `o|`/`|o` (zero or one), `}|`/`|{` (one or many), `}o`/`o{` (zero or many)
- Entity attributes
- Relationship labels

## Unicode Support

Full-width characters (CJK, emoji) are handled correctly in layout calculations.

## License

MIT
