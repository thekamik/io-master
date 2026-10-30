# IO Master: the data model

The models of `io-master/src/model/`, agreed with the user model by model, and their tables in `db/` (`users/<id>.v2.db`; `User`, `Installation` and `UserKeys` in the system database `system.v2.db`). **A model is changed only with the user's consent** (`CLAUDE.md`).

## 1. The rules of a model (user, 2026-10-28)

- **A model is a structure that represents the data kept in the database**: fields only, each commented, no functions. The rules of the data (checks, the order of the work) live outside `model/`.
- **Objects are named by their id in the database** (`i64`), never by a text.
- **One piece of information has one place.** What can be computed from the data is not stored beside it (no field of the kind of a work unit: `input_functional` tells it; no hash of a file: it is computed from the file).
- **Several plans, one table.** The work units of all plans are rows of one table, each naming its plan; no table is made while the program runs.
- **The names are those of the standards.** A work unit is named only by its label in the standard ("ADV_ARC.1-1"): ISO/IEC 17025 asks for an unambiguous identification, so the laboratory gives no names of its own.

## 2. The models and their links

```text
model/evaluation/                                   model/ai/
─────────────────                                   ─────────
EvaluationPlan                                      Integration ◄── ApiKey (integration)
  ├── oscal_documents ──────► OscalStandardDocument      ▲
  ├── product_documents ────► ProductDocument            │ integration
  └── work_units ───► WorkUnit                         Agent ──knowledge_bases──► KnowledgeBase ◄── Source
        WorkUnit                                         ▲                                       (knowledge_base)
          ├── reference ────► OscalStandardDocument       │                                       text ──► index (db/)
          ├── dependencies ─► WorkUnit (same plan)        │
          ├── input_product_documents ─► ProductDocument  │
          ├── input_functional ────────► LabProof          │
          └── ai_agent ────────────────────────────────────┘
                                                    JournalEntry (agent ──► Agent, work_unit ──► WorkUnit)
                                                    Chat (agent ──► Agent, work_unit ──► WorkUnit)

model/user/
───────────
User, Installation (system database, not encrypted)   Settings (active_integration ──► Integration)
```

### 2.1 `model/evaluation/`

| Model | File | Fields | Table |
|---|---|---|---|
| `EvaluationPlan` | `evaluation_plan.rs` | id, name, description, state, source_plan, product, client, evaluator, started, oscal_documents, product_documents, possible_verdicts, work_units, blocked, planning_agent | `evaluation_plans` (lists: `evaluation_plan_standards`, `_documents`, `_verdicts`) |
| `PlanState` | `evaluation_plan.rs` | Plan, Evaluation, Report | (a column) |
| `WorkUnit` | `work_unit.rs` | id, reference, label, oscal_key, requirement, guidance, dependencies, cross_reference, input_product_documents, input_functional, ai_agent, findings, analysis, vulnerability_notes, notes, quotes, verdict, author | `work_units` (lists: `work_unit_dependencies`, `_documents`, `_proofs`, `_quotes`) |
| `Quote` | `work_unit.rs` | product_document, text | `work_unit_quotes` |
| `Verdict` | `work_unit.rs` | Pass, Fail, Inconclusive | (a column) |
| `Author` | `work_unit.rs` | Ai, User | (a column) |
| `OscalStandardDocument` | `oscal_standard_document.rs` | id, title, doc, version | `oscal_standard_documents` |
| `ProductDocument` | `product_document.rs` | id, name, content, version | `product_documents` |
| `LabProof` | `lab_proof.rs` | id, name, proof, performed, tester, equipment | `lab_proofs` |

### 2.2 `model/ai/`

| Model | File | Fields | Table |
|---|---|---|---|
| `Integration` | `integration.rs` | id, name, url, model, protocol, ca_certificate, context_size | `ai_integrations` |
| `Protocol` | `integration.rs` | OpenAiChat, AnthropicMessages | (a column) |
| `ApiKey` | `api_key.rs` | integration, key | `ai_api_keys` (with the origin it is bound to) |
| `Agent` | `agent.rs` | id, name, integration, instructions, knowledge_bases, glossary, max_passages, max_characters | `ai_agents` (lists: `ai_agent_knowledge_bases`, `ai_agent_glossary`) |
| `GlossaryEntry` | `agent.rs` | term, meaning | (rows of the agent) |
| `KnowledgeBase` | `rag.rs` | id, name, description | `knowledge_bases` |
| `Source` | `rag.rs` | id, knowledge_base, name, text | `knowledge_sources` (index: `knowledge_passages`, FTS5) |
| `JournalEntry` | `journal.rs` | id, time, integration, model, agent, work_unit, request, answer, ending, input_tokens, output_tokens | `ai_journal` |
| `Ending` | `journal.rs` | Complete, Cut, Stopped, Refused, Failed | (a column) |
| `Chat` | `chat.rs` | id, title, agent, work_unit, messages | `chats` |
| `Message` | `chat.rs` | speaker, text, time | `chat_messages` |
| `Speaker` | `chat.rs` | User, Ai | (a column) |

### 2.3 `model/user/`

| Model | File | Fields | Table |
|---|---|---|---|
| `User` | `user_data.rs` | id, name, created | `users` (`system.v2.db`) |
| `Installation` | `program_data.rs` | id, created | `installation` (`system.v2.db`; create and read only) |
| `Settings` | `program_data.rs` | active_integration | `settings` (one row) |
| `UserKeys` | `user_keys.rs` | user, password (`WrappedKey`), recovery (`RecoveryWrappedKey`) | `user_keys` (`system.v2.db`) |
| `WrappedKey` | `user_keys.rs` | salt, memory_kib, iterations, nonce, ciphertext, tag | (columns of `user_keys`) |
| `RecoveryWrappedKey` | `user_keys.rs` | salt, nonce, ciphertext, tag | (columns of `user_keys`) |

## 3. The evaluation plan

- **A plan and its copies (user, 2026-10-28).** The planner makes a plan once (for example for routers, which share their tests); every evaluation of a product works on an identical copy of it. In the plan the results of the work units are empty and the documents of the product and the proofs of the laboratory are only places reserved with their ids and names ("ST", "IXIT", "results of the scan of the ports"); in the copy the evaluator gives the files and the texts and the work units are assessed. The field `state` tells where a plan is in its life (user, 2026-10-28): `Plan` while it is being planned, `Evaluation` while the evaluation is being carried out, `Report` when its last work unit is assessed: **the report is the filled plan**, no model of its own.
- **Every check is an evaluation (user, 2026-10-28).** A new version of a document of the product (a new ST) is a new copy of the plan, a new evaluation: the earlier checks stay as they were, with their documents and results.
- **The copy of a plan** gives its work units new ids; their `dependencies` are written anew with the new ids.
- **How an evaluation begins (user, 2026-10-28).** The plan stays in the database as it is. In the view of the evaluation the user looks through the plans and chooses one; the program makes its copy, at once in the state `Evaluation`, with `source_plan` naming the plan and the identification of the evaluation ISO/IEC 17025 asks for: `product`, `client`, `evaluator`, `started` (empty in a plan).
- **One station, one user, both roles (user, 2026-10-28).** The same user makes the plans and carries out the evaluations on one station; nothing passes between installations.
- **Two kinds of documents, never mixed.** The standards (`OscalStandardDocument`: the CEM, PP1, PP2) are the reference, kept as their OSCAL JSON in the database; the documents of the product (`ProductDocument`: the ST, the IXIT) come from the client, their files kept in the database, encrypted with it, so that what was assessed cannot change after the assessment. A proof of the laboratory (`LabProof`) keeps the record of its test that ISO/IEC 17025 asks for: when, by whom, with which equipment.
- **A plan declares its standards.** The `reference` of every work unit is one of the `oscal_documents` of its plan (a plan may hold the CEM and two protection profiles; a work unit belongs to one of them), and its `input_product_documents` are among the `product_documents` of its plan. `reference` with `oscal_key` (the `id` of the control in the JSON of the catalog) is the place of the work unit in its standard; `label` is its number in the standard.
- **A work unit of the documentation** is one whose `input_functional` is `None`: it needs no results of functional tests.

### 3.1 The dependencies

- The `dependencies` of a work unit are work units of the same plan, each named once, never the work unit itself.
- **A cross reference** is a cycle of dependencies (A depends on B, B on A, directly or through others), a mistake of the planner. The work units that close a cycle get `cross_reference = true`, and the plan with at least one of them `blocked = true`: it cannot be carried out until the planner corrects it. Both fields are set by one function after every change of the dependencies of a plan, so that they never disagree with them.
- **The agent of the planner** (`planning_agent`, user 2026-10-29) is an agent named by a plan in planning: it fills the parameters of the work units of the plan with its knowledge ("Fill with AI"). It does not go into an evaluation: the copy has `None`, and the database refuses an agent of the planner on a plan that is no more in planning.
- **The order of the work** comes from the same function (Kahn's algorithm, without recursion): a work unit comes after all its dependencies, otherwise in the order of the plan. What Kahn's algorithm leaves depends on a cycle; the same algorithm backward on it leaves only the work units of the cycles themselves, which are the cross references (a work unit that only depends on a cycle is not marked). The function is `panels/logic/evaluation_plan/order.rs` (user, 2026-10-29: not a part of `db/`), called after every change of the work units of a plan; the copy of a plan into an evaluation is refused while the plan is blocked or a work unit is marked.
- **A work unit is removed** only from a plan in planning (`delete_work_unit`, user 2026-10-29), with the dependencies of the others on it.
- **A new work unit** is made from a control of a standard of its plan: its label, its key, the text of its statement (the requirement) and of its guidance; no documents, proofs or agent yet, which the planner gives, by hand or through the agent of the planner ("Fill with AI"). A work unit becomes a functional test when it gets a proof of the laboratory.

### 3.2 The verdict (user, 2026-10-28, as Common Criteria asks)

- **The verdicts of a plan.** Common Criteria knows only `Pass` and `Fail`; ETSI EN 303 645 also `Inconclusive`. A plan names those its standards know (`possible_verdicts`): `Pass` and `Fail` when it is made; the planner may add `Inconclusive`. From the worst to the best: `Fail`, `Inconclusive`, `Pass`.
- A work unit is assessed by the AI even when a dependency did not pass, so that all its faults are found for the Observation Report.
- Its verdict is the worst of its own and the verdicts of its dependencies: a work unit that depends on one `Fail` is `Fail`.
- `findings` holds every fault: those the AI found, and each dependency that did not pass, written by the program ("fail: its dependency ADV_ARC.1-1 failed").
- **The quotes (P5).** The assessment rests on quotes of the documents of the product, word for word, each with the document it is taken from (`Quote`); whether each stands in the documents is checked by the program, never taken from the AI, so that a fault of the document is told from an invention of the model.
- **The author (P1).** `author` tells whether the results and the verdict are those the AI proposed, or the user wrote or changed them.

## 4. The AI

- **An integration** is where the program sends its questions: the address, the model, the protocol (the chat completions of OpenAI, which most local servers offer, or the messages of Anthropic), the trusted CA certificate, and the size of the context of the model: a question that does not fit is refused before it is sent, never shortened in silence. **Its API key** is kept apart, only in the encrypted database, never printed (no `Debug`) and never copied (no `Clone`).
- **An agent** is a layer of knowledge (RAG) over the model of an integration (user, 2026-10-29): it names the one integration that answers its questions (one integration serves many agents), its instructions, its knowledge bases (shared with other agents), its glossary and its limits. It is made in its own panel, changed when its answers are not good enough, and named by the work units it works on (one agent may serve several). In an evaluation the question of a work unit goes through its agent: the agent adds its knowledge as the context, and the answer comes back to the work unit.
- **The knowledge (RAG)** is a search before the question, not a training of the model. A knowledge base is a folder of sources, shared by the agents that name it; a source is one text (an earlier evaluation, a note). Whenever a text is stored, `db/` cuts it into passages of a bounded length and indexes them (FTS5); at a question the passages of the bases of the agent that fit it best are given to the model with the question; the common words of English ("is", "the", "shall") are passed over in the search. The passages are an index, made again from the text, not a model.
- **A chat** is the place where the user talks with an agent they choose (user, 2026-10-30): the model of its integration answers, with the knowledge of the agent found for each question (looked for with the question and the question of the user before it). A chat always names its agent; it is never removed, its title may be changed. An answer stopped, cut or refused is kept with a mark after its text ("[stopped]", "[cut]", "[refused]"); a failed one is not kept in the chat, its reason is in the journal of the agent.
- **The journal of the AI** keeps every question of an agent and its answer, only ever added to, with the work unit it was asked for: every result of the AI can be traced to what it rests on. **Each agent has its own journal**, the entries that name it (user, 2026-10-29): every entry names its agent by its id. A question without an agent (the check of an integration) is not kept: it is no part of an evaluation. The integration and the model are kept by their names, so that an entry stays as it was.

## 5. The user and the program

- One Linux account holds one installation with one user. `User` and `Installation` are kept in the system database, which is not encrypted and holds nothing secret; the copies of the key of the encrypted database of the user are kept beside the user but belong to `db/encryption/` (`architecture.md`, section 5).
- **The keys of a user** (`UserKeys`, user 2026-10-28) are a model of their own, apart from `User`: the two copies of the one random key of the database of the user, wrapped with the password (Argon2id) and with the recovery code, each bound to the id of the user (`panels/logic/program_start/access.rs`). None of it is secret. The two copies keep the shape they had in the earlier program.
- `Settings` names every setting as a field of its own: today the active integration (its use is open, `todo.md`).

## 6. The standards in OSCAL

- A standard is an **OSCAL 1.2.3 catalog** (`OscalStandardDocument::doc`, its JSON), checked against the schema of NIST built into the program (`io-master/oscal/`) when it is imported; its title and version are read from its metadata.
- A work unit is made from one control of a catalog of its plan: `oscal_key` is the `id` of the control, `label` its label in the standard (the `prop` `label`), `requirement` and `guidance` the text of its statement and guidance parts.
- The plan and the evaluation are the program's own model, linked to the catalogs only by these ids: nothing of ours is written into an OSCAL document.
