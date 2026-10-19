# P8 developer command

Status: contract `blocks/cli.md`; implementation open.

## Exit checklist

`blocks/cli.md` §7.

- [ ] 1. `check` over the accept and reject corpus
- [ ] 2. `run` over `examples/typescript/` and `examples/run/`
- [ ] 3. `run` on trap entries
- [ ] 4. `build --run` on `examples/hello/` and `examples/run/`
- [ ] 5. `link-flags` names an existing library; one flag definition
- [ ] 6. Unit tests of §2 to §5
- [ ] 7. `gate.sh full` and `hygiene.sh` pass; no golden changes
- [ ] Phase Review

## Measurements

None yet.

## Failures before the fix

None yet.

## Gaps

- `run` and `check` take no grants. A script that calls a host
  function needs its host.
