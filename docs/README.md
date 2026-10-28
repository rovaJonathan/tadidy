# docs/

| Folder | Content | Naming |
|---|---|---|
| [spec/](spec/README.md) | The specification, one part per file, + the one-page [spec.html](spec/spec.html) — **source of truth** | `NN-<part>.md`, stable `<a id>` anchors |
| [prototype/](prototype/prototype.html) | Clickable reference prototype of every screen — **source of truth** for UI texts, ids and states | — |
| [adr/](adr/README.md) | Architecture decisions | `NNNN-<slug>.md` |
| [plans/](plans/) | Plan for non-trivial work, written before coding | `YYYY-MM-DD-TAD-xxx-<slug>-plan.md` |
| [solutions/](solutions/) | Solved problems that will come back: tool traps, obscure errors, workarounds | `<category>/YYYY-MM-DD-<slug>.md` |
| [runbooks/](runbooks/) | Operating procedures: signing identity, release, data recovery | `<slug>.md` |
| [reference.md](reference.md) | Versions, tools, paths, bundle identifiers | — |
| [guides/](guides/) | For humans and agents: [tickets](guides/tickets.md), [manual QA](guides/manual-qa.md) | — |
| [archive/](archive/) | The French first draft of the spec — **no longer authoritative** | — |

The visual system is in [../DESIGN.md](../DESIGN.md), the vocabulary in [../GLOSSARY.md](../GLOSSARY.md), the tickets in [../tickets/](../tickets/README.md).

## Rules

- **Plans**: at the top, the ticket (`TAD-xxx`), the goal, the files touched, the steps, the risks and the planned verification. Check a box only when it is done and verified. An abandoned plan moves to `plans/archive/` with one line saying why.
- **Solutions**: YAML header `title`, `date`, `area` (`core` | `ui` | `repo`), `tags`, then Symptom · Cause · Fix · How to avoid it. Search here before debugging for long.
- **Runbooks**: Scope and symptoms · Prerequisites · Bounded steps · Verification · Rollback. Separate diagnosis (read-only) from action.
- **Spec**: never delete or rename an anchor a ticket cites (`node scripts/tickets.mjs check` fails if you do). After editing, rebuild the HTML: `mise exec pandoc@3.1.11.1 -- scripts/spec/build.sh` (or any pandoc 3).
