# Agent workflow

`CLAUDE.md` defines the roles. This file defines the handoff and review
formats.

## Implementation handoff

A handoff states each field below. Give the agent the contract and the
relevant corpus entries, not the planning conversation.

```text
Objective:
Contract and section:
Owned files:
Corpus entries (accept, reject, trap):
Expected failure before implementation:
Verification command:
Environment: LUAU_SOURCE_DIR=<value from the session, never committed>
Excluded scope:
Rules: do not edit specs/, do not commit, do not detach processes,
  do not start reviewer agents.
Return: changed files, command results, remaining gaps.
```

If the contract does not state a fact that the implementation needs,
the agent reports the missing fact and stops that part. The primary
agent updates the contract.

## Phase review handoff

```text
Phase and contract sections:
Diff range (git revisions) or initial file inventory:
Verification commands:
Threat model: CLAUDE.md invariant 2; severity follows it.
Return: findings as CRITICAL, MAJOR, or MINOR, each with location,
  evidence, failure mode, and a reproduction.
```

The reviewer reads the implementation, not only the diff summary. It
does not edit files.

## Tracking note

One note per slice in `specs/tracking/<topic>.md`. Sections: status,
exit checklist, measurements, failures before the fix, gaps. Record
commands and numbers. Do not record narrative.
