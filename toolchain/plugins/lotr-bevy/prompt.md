# {{PROJECT_NAME}} contributor prompt

You are recovering the 32-bit SAGE executable for **The Lord of the Rings:
The Battle for Middle-earth II, patch 1.06** as safe, deterministic Rust.
Work is coordinated through {{REPOSITORY_URL}} under policy
v{{POLICY_VERSION}}. Only merged pull requests count.

## Readiness gate

Before claiming work, run:

```bash
recomp task doctor --project . --target /path/to/game.dat
```

Stop unless the last line is `GREENLIGHT`. If the target check fails, tell the
human that the required file is `game.dat` from patch 1.06, exactly 10,969,600
bytes, SHA-256 `{{TARGET_SHA256}}`. Never copy the target into this checkout.

## Claim, recover, publish

```bash
recomp task next {{REPOSITORY_URL}}
recomp task start <issue-url> --target /path/to/game.dat
recomp task run
# edit only the declared Rust output module
recomp task verify
recomp task submit
```

`start` waits for the coordinator App's trusted two-hour lease. Use
`recomp task heartbeat` to renew it. One claim, one output module, one pull
request, and exactly one `Closes #<issue>` reference.

Each Issue contains one batch of measured, oracle-callable x86 functions. For
every listed address:

1. Read `reference/decomp/<address>.c` and the signature comment in the
   generated `recovery/crates/sage-core/src/recomp_tasks/port_NNNN.rs`.
2. Add the address to `call`'s match and implement the same behavior in safe,
   deterministic Rust. `args` contains raw ABI bits in declaration order:
   integer-like values in the low 32 bits, `f32` as IEEE-754 low-32 bits, and
   `f64` as all 64 bits. `scratch` is the original function's 64-byte `this`
   object image and may be read or mutated.
3. Return `Observed { eax: Some(value), double_bits: None }` for
   integer/pointer-like returns; `Observed { eax: None, double_bits:
   Some(f64_bits) }` for floating returns; both `None` for `void`.
4. Run `recomp task verify`. The local container compares compiled Rust with
   Unicorn execution of the pinned original. The coordinator repeats candidate
   and oracle execution in separate networkless Firecracker guests; candidate
   code never receives `game.dat`.

Do not fake expected hashes or special-case public inputs. Do not add target
bytes, extracted resources, generated goldens, local paths, build outputs, or
dependencies. Keep the module deterministic and format it with `rustfmt` if
available. If decompilation is ambiguous, report the address and evidence in
the pull request rather than guessing.

Never edit `.github/`, `plugins/`, `toolchain/`, `recomp.toml`,
`recomp.lock.toml`, `recomp-launcher`, `docs/progress.*`, or `AGENTS.md`.
Stale-policy preflight means rebase and re-claim under the current policy.

When stopping, report tasks claimed/merged/rejected, policy
v{{POLICY_VERSION}}, your real model name, minutes per task, and abandoned
tasks with reasons.
