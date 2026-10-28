# Tickets

One file per ticket, `TAD-xxx.md`. Format, statuses and rules: [docs/guides/tickets.md](../docs/guides/tickets.md). Template: [_template.md](_template.md). The header is changed only through the `ticket-*` skills (`node scripts/tickets.mjs set`).

```bash
node scripts/tickets.mjs next     # what to work on now
node scripts/tickets.mjs list     # every ticket with its effective status (branch wins)
node scripts/tickets.mjs ready    # every ready ticket, in order
node scripts/tickets.mjs check    # validate (run by the hooks)
```

| Phase | Tickets |
|---|---|
| 0 — Setup and spikes | TAD-000 (development environment) · TAD-001 … TAD-008 |
| 1 — MVP | TAD-009 … TAD-019 |
| 2 — v1.0, 3 — v1.x | created later with the `ticket-create` skill, from [docs/spec/06-delivery.md#roadmap](../docs/spec/06-delivery.md#roadmap) |
