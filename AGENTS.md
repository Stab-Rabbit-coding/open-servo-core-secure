# AGENTS.md — authoritative instructions for all AI agents

Model-agnostic project instructions. Every AI assistant working in this
repository — Claude, Gemini, Grok, Copilot, or any other — follows this file.
`CLAUDE.md`, `.github/copilot-instructions.md` and `.zed`/`.vscode` stubs all
point here.

## What this project is

Open Servo Core: firmware, hardware and mechanics for a smart servo actuator
built on a sub-$0.20 MCU (CH32V006, RV32EC, 48 MHz, 62 KB flash, 8 KB SRAM).
The `-secure` fork adds a secure element and per-message bus authentication.

**Every design decision is for an actual functional build.** Parts get
fabricated or procured. Size fasteners, walls, traces and buffers for real
loads; quote real masses, currents and timings. Nothing is left "TBD" without
a `TODO.md` item against it.

## Authoritative documents

Read these before changing anything they govern. **Code is authoritative; when
a doc and the code disagree, fix the doc.**

| Document | Governs |
| -------- | ------- |
| `docs/osc-native-protocol.md` | the wire protocol — normative |
| `docs/osc-servo-transport.md` | the servo-side transport |
| `docs/control-theory.md` | the control cascade |
| `docs/driver-pattern.md` | crate layering and the provider pattern |
| `docs/security-architecture.md` | the secure element and message plane |
| `REFERENCES.md` | every external standard, datasheet and source |
| `TODO.md` | the work breakdown structure |
| `PROJECT_INDEX.md` | the active file tree |

## Authenticity rules — non-negotiable

- **No reference, citation, standard, section number, constant or measurement
  is ever fabricated.** If a value cannot be traced to a published document
  with a validated URL, omit it or mark it *requires verification* in
  `REFERENCES.md` and open a `TODO.md` item.
- A plausible-looking guess in a driver or a security primitive is **worse
  than an absent one**: it compiles, runs, fails obscurely, and looks
  authoritative while doing it. Prefer a compile gate over a stub.
- **Timing claims are silicon-measured**, and catalogued as `[Fn]` facts in
  `osc-native-protocol.md` §11. Analytical estimates are permitted only when
  explicitly labelled as such, with a bench probe and a `TODO.md` gate.
- **Every external idea is cited** in the source file's doc comment and in the
  commit message, meeting or exceeding CC-BY-4.0 attribution requirements.
  Derivative files carry the full attribution chain to upstream.
- **AI work is distinguished from human work.** Human contributors are
  referenced by GitHub username. Each AI model is cited for its own
  contribution — Claude Opus 5 separately from Fable 5, Gemini separately from
  Grok.

## Standards vetting

Any specification with an effect beyond cosmetic appearance is vetted against
applicable industry standards before implementation, and recorded in
`REFERENCES.md` with its designation, validated URL, the specific section
applied, and every repository location that cites it. Use the REF-ID in code.
Never invent a section number.

US jurisdiction for all legal and regulatory questions.

## Engineering conventions

- Measurements are **imperial-primary with metric in parentheses**:
  `10 in (254 mm)`, `2.5 lbm (1.13 kg)`, `4.8 lbf (21.4 N)`. Use **lbm** for
  mass and **lbf** for force; never bare "lb". Airspeed in **knots (kt)**.
- Electrical, timing and memory quantities use their native SI units — this is
  an embedded project and µs, MHz, mA, KB are the working vocabulary.
- Account for power, timing, flash and RAM budgets explicitly. The app image
  has ~57 KB of flash and 8 KB of SRAM; say what a change costs.

## Coding standards

- Language preference: Python3, C++, Bash, JavaScript. This repository is
  **Rust** (`no_std`, edition 2024) plus Python tooling.
- **4-space indent in every language**, whether or not the language requires
  it.
- Secure coding practices throughout. Run static analysis (`cargo clippy`)
  before committing.
- Strict linting everywhere, including all Markdown rules.
- Verbose commenting, in each language's idiom. Match the surrounding code's
  comment density and voice — this codebase explains *why*, not *what*, and
  cites the governing doc section.
- For formats without inline comments (KiCad), put the commentary in an
  accompanying Markdown file.
- All PRs pass CI: at minimum a security check and a lint check per language.

## Workflow

- Any multi-step task an agent plans gets its steps added to the appropriate
  `TODO.md` WBS paragraph, so unresolved work survives the session.
- Update `PROJECT_INDEX.md` when active files are added; move archived names
  to `ARCHIVE_INDEX.md`.
- Mirror auto-generated agent memory to `<AGENT>-MEMORY.md` in the repo root
  for auditability.
- **Never leave a `TODO.md` checkbox open once its own text says the item is
  resolved or superseded.** Close it out in the same edit.
