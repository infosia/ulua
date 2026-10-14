# CLAUDE.md — Ulua permanent development rules

This file holds only what is *invariant*: roles, boundaries, and
conventions. `specs/plan.md` holds design and phasing.
`specs/rules-history.md` holds the date and the evidence of each rule
here. When the plan and this file disagree, this file wins. When
evidence disagrees with either, fix both.

## What this project is

**Ulua** is an embedded scripting language for native host applications.
Its syntax is a TypeScript subset. A Rust compiler emits typed Luau
source. The pinned Luau compiler and VM run that source. The host owns
its main loop and calls exported script functions through a C ABI with
the prefix `ulua_`.

## Roles (read first)

The primary agent (the Claude Code main session) plans and
orchestrates. It writes `specs/`, `corpus/` entries and goldens,
`tools/`, `docs/`, `README.md`, the scripts of `examples/`, and agent
configuration. It sends task handoffs, reviews
diffs against acceptance criteria, runs gates, and makes local commits.
It does not write production code under `crates/`, `native/`, or
`include/`, or the hosts of `examples/`.

The implementation agent is Codex, started as a fresh `codex:codex-rescue`
agent of the Codex plugin. It writes production code and tests within one handoff,
and the hosts of `examples/` (`host.c`) with their expected output. It
does not plan, edit `specs/`, change scope, review its own work, start
reviewer agents, or commit. A subagent that receives a handoff has this
role. The handoff format is in `specs/workflow.md`.

The reviewer is a fresh `general-purpose` subagent with no conversation
context. Never use a fork. The reviewer reads the diff and the contract
and does not edit files.

Run concurrent implementation agents only on disjoint files. Continue a
running implementation with SendMessage. Use a new agent for a new
slice. An agent never detaches a process (`nohup`, `setsid`, a trailing
`&`). It waits for each gate before it reports.

## Design invariants (read second)

1. **Embedded by construction.** The host owns the main loop. Host
   interop crosses the C ABI only. A script reaches the host only
   through capabilities that the host grants explicitly.
2. **Scripts are hostile. The host is trusted.** Every script input is
   a realistic reach. Compilation and execution stay inside limits that
   the host sets: source size, nesting depth, time, and memory. The
   compiler adds fixed bounds that keep generated Luau inside the Luau
   limits.
3. **The syntax is a valid-TypeScript subset.** Every accepted program
   type-checks under stock `tsc` with the Ulua prelude and the host
   declarations that the library writes for its grants. The Ulua
   compiler then narrows the accepted set. A `tsc` resource limit
   (TS2563, TS2321, a `tsc` stack overflow) is outside this rule: `tsc` has
   its own limits, as `node` has under invariant 5. At an equality
   expression that the compiler accepts by its type rule
   (`collisions.md` C7), `tsc` can give TS2367 or TS2845; these two
   codes at that position are outside this rule. After such an
   equality, a null test (`collisions.md` C2), or an assignment of
   `null`, `tsc` can narrow a reference to `never`. A `tsc`
   diagnostic whose message names the type `never` is outside this
   rule.
   Every other `tsc` diagnostic fails the program.
4. **Values keep their native Luau representation.** A value of
   TypeScript type `T` is the plain Luau value of the Luau type that
   `T` lowers to. A lowering can replace an operation with a fixed Luau
   expression. A lowering never changes how a value is represented: no
   sentinels, no re-encoded strings, no length fields beside arrays.
5. **JavaScript agreement is the default. A divergence is decided.**
   An accepted program prints the output that `node` prints, unless
   `collisions.md` names a divergence for its form. A divergence entry
   states the measured outputs of both sides. A disagreement that no
   rule names is a defect: the compiler rejects the form, or the
   contract names the divergence. A trap where `node` produces a value
   outside the static type is a named divergence. A limit diagnostic
   and a resource-limit trap (invariant 2) are outside the comparison:
   `node` has its own, different limits.
6. **One pipeline.** The Rust compiler emits Luau source with
   `--!strict` type annotations for the pinned Luau compiler. Do not
   emit bytecode. Do not add a second backend. Generated Luau and
   bytecode stay internal. No public API accepts raw Luau or bytecode.

## Non-goals (permanent unless the plan is revised with evidence)

- **npm compatibility and existing TypeScript code.** The subset rejects
  each form that neither agrees with `node` nor has a named divergence
  (invariant 5).
- **Emulation of JavaScript semantics.** No runtime layer imitates a
  JavaScript value representation (invariant 4).
- **Upstreaming to external projects.** When this project must change a
  dependency, it forks and pins the fork. It does not open pull
  requests. Forks are cited by URL and pinned by commit.
- **A standalone program runtime.** The host owns the loop. No event
  loop and no background scheduler run script code.

## Oracle and witnesses

The oracle is the committed golden corpus outputs. Four external
witnesses are divergence detectors, never oracles:

- `tsc` gates acceptance. Every accept entry type-checks. Every reject
  entry's header states what `tsc` does, measured.
- `node` runs each accept and trap entry with the header
  `js-comparable: yes`. A disagreement is a defect of this compiler or
  a divergence that `collisions.md` must name (invariant 5). A
  disagreement never corrects a golden. An entry with
  `js-comparable: no` cites the collision id that names its
  divergence.
- `luau-analyze` in strict mode checks the generated Luau with zero
  errors.
- The Luau VM runs the generated Luau through the product C ABI.

A trap entry compares the output before the trap, and the trap kind.

## Language

- All repository documentation, specs, comments, and identifiers:
  English.
- Conversation with the user (chat responses): Japanese.

## Writing style

Applies to specs, tracking notes, comments, commit messages, and chat
reports. **State fact, evidence, and consequence. Nothing else.**

Forbidden: rhetorical or dramatic framing; narrating a mistake as a
story; restating a lesson in more than one document; self-referential
commentary; emphatic repetition.

Required: write a lesson once, as a one-line rule. A report states what
changed, what was measured, and what is next. A correction states what
was wrong, the evidence, and the corrected claim.

**A claim about another system's behaviour requires running that
system.** Mark a claim taken from documentation alone with *(docs)*.

**A rule here carries no date and no origin.** The date and the
evidence of a rule are in `specs/rules-history.md`. The evidence of a
contract section is in `specs/tracking/`.

### Simplified Technical English

Write all English in ASD-STE100 Simplified Technical English
(<https://www.asd-ste100.org/>), pragmatic mode: apply the structural
rules and keep the domain vocabulary.

- Maximum 20 words in an instruction. Maximum 25 in a description.
- One word, one meaning, for the whole document.
- Simple tenses only. Do not write "has been" or "is being".
- Do not use an `-ing` form as a verb.
- Use the active voice.
- `can`, `will`, and `must` are approved. `should`, `would`, `may`,
  `might`, and `could` are not. Write a suggestion as a fact, or delete
  it.
- Put the condition before the command.
- Write one instruction per sentence.
- Keep the articles and keep "that".
- For a risk, give the command first and the consequence second.

Never rewrite code, identifiers, commands, or quoted error text.
Japanese chat replies follow the same intent: short sentences, one term
for one concept, no hedging.

## Core principles

1. **Every public API has a direct unit test**, in the same commit.
2. **Corpus-first.** `corpus/accept/`, `corpus/reject/`, and
   `corpus/trap/` are the executable definition of the language. A
   syntax or semantics decision without a corpus entry is not decided.
3. **Every gate run compares every corpus entry with its golden** and
   with each witness above that applies to the entry.
4. **Headless-first.** Every gate passes with no GPU, no window, and no
   device.
5. **No panics in library code.** Use `Result` and `?`. No panic and no
   C++ exception crosses the C ABI.
6. **Generated code is never hand-edited.** Fix the generator.
7. **Exit criteria before implementation.** Each phase names, in
   advance, the measurement that passes or stops it.
8. **A form carries every fact its consumers need.** A stage is a total
   function from its input form to its output. If a consumer needs a
   fact the form does not carry, the form is wrong. Report it and stop.
9. **A check compares two facts that were derived separately.** A check
   that reads a record against the expression that wrote it cannot
   fail. A test builds the violating form.
10. **A corpus entry is Red at the contract pin.** Verify the failure
    against a binary built from that pin.
11. **Move one consumer at a time** during a migration.
12. **A defect that all witnesses share is invisible to them.** Pin such
    a shape with a reviewed golden or a hand-checked value.
13. **A change needs a stated problem and evidence that the problem is
    real.** Refuse a proposal that states no problem, whoever writes it.
14. **A record is not a reason.** "The contract says so" and "a golden
    pins it" are evidence of a past answer, not of a correct one. Ask
    what the form must be first. Then read the record to learn the
    cost of the change.
15. **A gate's cost is part of its design.** A gate test states what it
    costs, and the cost is measured. A cost that no reason covers comes
    out before the test lands. A gate assertion never depends on
    wall-clock time or machine load: assert ratios, not budgets.
16. **A hidden cost is rejected.** If the cost of a form depends on
    something that the source does not show, reject the form with a
    diagnostic that names the cause.

## Code conventions

- `#[non_exhaustive]` on extensible public enums and structs.
- `///` docs on every public item, with `#![warn(missing_docs)]`.
- Every `unsafe` block and every `unsafe impl` carries a `// SAFETY:`
  comment.
- An `#[allow]` on a correctness lint requires a justifying comment.
- A source file has at most 1,000 lines. Split by responsibility.
- The tree is rustfmt-canonical under the toolchain that
  `rust-toolchain.toml` pins. `cargo fmt --check` is a gate.

## Workflow per area

1. Write or extend `specs/blocks/<area>.md`: contract first.
2. Add corpus entries (accept, reject, trap): Red.
3. Implement: Green.
4. Run `sh tools/gate.sh quick`.
5. Record evidence in `specs/tracking/<topic>.md`: status, exit
   checklist, measurements, failures before the fix, gaps.
6. Commit locally. Never push. The user runs network commands.

**Step 0: if the question is whether a rule must change, measure
first.** A measurement round builds a prototype, records its numbers,
and reverts every production change. It can contradict a stated rule.
The tracking note names that rule beside the numbers.

**Solve the class that a defect shows. Do not count rounds.** If a
review finds a defect, first ask whether the current form can close its
class. If it cannot, change the form or add a total check, contract
first.

**Every phase ends with a Phase Review.** A fresh reviewer reads the
cumulative diff of the phase and reports `CRITICAL`, `MAJOR`, or `MINOR`
findings with location, evidence, and failure mode. Fix findings in
severity order. A phase cannot be COMPLETE with an open CRITICAL or
MAJOR finding. Then run `sh tools/gate.sh full` and `sh tools/hygiene.sh`.

**Severity follows invariant 2.** Any script input is a realistic
reach. A finding that needs the trusted host to break a documented
precondition is `MINOR` at most.

**At the end of each phase, account for each task that is still
running.** List each background shell, monitor, and subagent that the
session started, what it waits for, and who reads its result. If no
reader exists, stop it.

## Privacy and repository hygiene

- Commit no credentials, signing material, or device-specific secrets.
- **Nothing committed references a path outside this repository.** This
  applies to every tracked file and to commit messages. Forbidden:
  absolute paths into a developer filesystem; relative paths that
  escape the repository root; machine- or user-specific names;
  predecessor or sibling projects by name or path.
- Cite an external project by its upstream URL and its own
  repository-relative paths. Pin external sources by revision.
- `tools/hygiene.sh` scans the working tree for these rules and exits 1
  with every hit.

## Environment

- `LUAU_SOURCE_DIR` names a local Luau checkout at the revision in
  `luau-revision.txt`. Pass it to the implementation agent in each
  handoff. Never write its value into the repository.
- Run long builds in the background. Do not poll them with sleep.
- Keep scratch files in the session scratchpad, not in the repository.
- Do not disable the sandbox for network operations. The user runs
  `npm install`, `cargo fetch`, and `git push` through the `!` prompt.
