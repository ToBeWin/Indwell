# Reflection Engine

The reflection layer converts episodic interaction records into candidate long-term memories and reusable skill templates.

Implemented:

- `ReflectionEngine`
- bounded `ReflectionBudget`
- sensitive-source skipping unless explicitly allowed
- preference derivation
- relationship note derivation
- emotional pattern derivation
- simple skill template generation for repeated-request patterns
- host simulator endpoint: `POST /v1/reflection/run`
- PWA Reflection panel

Reflection output is intentionally conservative:

- It does not invent memories without source records.
- Derived memories include `source:<record_id>` tags.
- Sensitive memories are skipped by default.
- Skill templates are data records, not executable scripts.

Next layers:

- Provider-assisted summarization with strict JSON schema.
- User confirmation UI for private/sensitive derived memories.
- Scheduled daily reflection budget.
- Guardian view and stricter retention for child/elder modes.
