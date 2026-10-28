# 0005 — English everywhere in the repository

- **Status**: accepted
- **Date**: 2026-10-28

## Context

The first draft of the spec was written in French. The app's UI is English only, and code, commits and pull requests read best in a single language.

## Decision

Everything committed is in English: code, comments, UI texts, spec, docs, ADRs, tickets, `PROGRESS.md`, commit messages, pull requests and review replies. The French draft is kept in `docs/archive/SPEC-fr.md` and is no longer authoritative. The owner may talk to the agent in French; the agent answers in the owner's language and writes the repository in English.

## Consequences

- One vocabulary ([GLOSSARY.md](../../GLOSSARY.md)) shared by spec, code and UI.

## Rejected alternatives

- **French docs and tickets, English code** (as in fluent-test): two languages to keep aligned; the UI texts would be English inside French tickets.
