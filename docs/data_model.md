# IO Master: the data model

**State (2026-10-28):** the models of `io-master/src/model/` are agreed with the user, model by model, and written as plain structures; **no code uses them yet**. The code is moved to them from `model_old/` step by step, so that the program works all the time (`code_structure.md`, section 2.1). The tables named here are the plan for `db/`; the tables of today are those of `model_old/`.

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
  ├── oscal_documents ──────► OscalDocument              ▲
  ├── product_documents ────► ProductDocument            │ integration
  └── work_units: Vec<WorkUnit>                        Agent ──knowledge_bases──► KnowledgeBase ◄── Source
        WorkUnit                                         ▲                                       (knowledge_base)
          ├── reference ────► OscalDocument               │                                       text ──► index (db/)
          ├── dependencies ─► WorkUnit (same plan)        │
          ├── input_product_documents ─► ProductDocument  │
          ├── input_functional ────────► LabProof          │
          └── ai_agent ────────────────────────────────────┘
                                                    JournalEntry (work_unit ──► WorkUnit)

model/user/
───────────
User, Installation (system database, not encrypted)   Settings (active_integration ──► Integration)
```

### 2.1 `model/evaluation/`

| Model | File | Fields | Table |
|---|---|---|---|
| `EvaluationPlan` | `evaluation_plan.rs` | id, name, description, oscal_documents, product_documents, work_units, blocked | `evaluation_plans` |
| `WorkUnit` | `work_unit.rs` | id, reference, label, oscal_key, requirement, guidance, dependencies, cross_reference, input_product_documents, input_functional, ai_agent, findings, analysis, vulnerability_notes, notes, verdict | `work_units` |
| `Verdict` | `work_unit.rs` | Met, NotMet, Inconclusive | (a column) |
| `OscalDocument` | `oscal_document.rs` | id, doc, version | `oscal_documents` |
| `ProductDocument` | `product_document.rs` | id, name, content, version | `product_documents` |
| `LabProof` | `lab_proof.rs` | id, name, proof | `lab_proofs` |

### 2.2 `model/ai/`

| Model | File | Fields | Table |
|---|---|---|---|
| `Integration` | `integration.rs` | id, name, url, model, protocol, ca_certificate, context_size | `ai_integrations` |
| `Protocol` | `integration.rs` | OpenAiChat, AnthropicMessages | (a column) |
| `ApiKey` | `api_key.rs` | integration, key | `secrets` |
| `Agent` | `agent.rs` | id, name, integration, instructions, knowledge_bases, glossary, max_passages, max_characters | `ai_agents` |
| `GlossaryEntry` | `agent.rs` | term, meaning | (rows of the agent) |
| `KnowledgeBase` | `rag.rs` | id, name, description | `knowledge_bases` |
| `Source` | `rag.rs` | id, knowledge_base, name, text | `knowledge_sources` |
| `JournalEntry` | `journal.rs` | id, time, integration, model, agent, work_unit, request, answer, ending, input_tokens, output_tokens | `ai_journal` |
| `Ending` | `journal.rs` | Complete, Cut, Stopped, Refused, Failed | (a column) |

### 2.3 `model/user/`

| Model | File | Fields | Table |
|---|---|---|---|
| `User` | `user_data.rs` | id, name, created | `users` (system database) |
| `Installation` | `program_data.rs` | id, created | `installation` (system database) |
| `Settings` | `program_data.rs` | active_integration | `settings` |

## 3. The evaluation plan

- **A plan and its copies (user, 2026-10-28).** The planner makes a plan once (for example for routers, which share their tests); every evaluation of a product works on an identical copy of it. In the plan the results of the work units are empty and the documents of the product and the proofs of the laboratory are only places reserved with their ids and names ("ST", "IXIT", "results of the scan of the ports"); in the copy the evaluator gives the files and the texts and the work units are assessed.
- **Two kinds of documents, never mixed.** The standards (`OscalDocument`: the CEM, PP1, PP2) are the reference, kept as their OSCAL JSON in the database; the documents of the product (`ProductDocument`: the ST, the IXIT) come from the client, their files kept in the database, encrypted with it, so that what was assessed cannot change after the assessment.
- **A plan declares its standards.** The `reference` of every work unit is one of the `oscal_documents` of its plan (a plan may hold the CEM and two protection profiles; a work unit belongs to one of them), and its `input_product_documents` are among the `product_documents` of its plan. `reference` with `oscal_key` (the `id` of the control in the JSON of the catalog) is the place of the work unit in its standard; `label` is its number in the standard.
- **A work unit of the documentation** is one whose `input_functional` is `None`: it needs no results of functional tests.

### 3.1 The dependencies

- The `dependencies` of a work unit are work units of the same plan, each named once, never the work unit itself.
- **A cross reference** is a cycle of dependencies (A depends on B, B on A, directly or through others), a mistake of the planner. The work units that close a cycle get `cross_reference = true`, and the plan with at least one of them `blocked = true`: it cannot be carried out until the planner corrects it. Both fields are set by one function after every change of the dependencies of a plan, so that they never disagree with them.
- **The order of the work** comes from the same function (Kahn's algorithm, without recursion, each work unit and each dependency visited once): a work unit comes after all its dependencies, otherwise in the order of the standard.

### 3.2 The verdict (user, 2026-10-28, as Common Criteria asks)

- A work unit is assessed by the AI even when a dependency is not met, so that all its faults are found for the Observation Report.
- Its verdict is the worst of the verdict of the AI and the verdicts of its dependencies: a work unit that depends on one `NotMet` is `NotMet`.
- `findings` holds every fault: those the AI found, and each dependency not met, written by the program ("not met: its dependency ADV_ARC.1-1 is not met").

## 4. The AI

- **An integration** is where the program sends its questions: the address, the model, the protocol (the chat completions of OpenAI, which most local servers offer, or the messages of Anthropic), the trusted CA certificate, and the size of the context of the model: a question that does not fit is refused before it is sent, never shortened in silence. **Its API key** is kept apart, only in the encrypted database, never printed (no `Debug`) and never copied (no `Clone`).
- **An agent** asks one integration, with its instructions, its knowledge bases, its glossary and its limits. A work unit names its agent.
- **The knowledge (RAG)** is a search before the question, not a training of the model. A knowledge base is a folder of sources, shared by the agents that name it; a source is one text (an earlier evaluation, a note). Whenever a text is stored, `db/` cuts it into passages of a bounded length and indexes them (FTS5); at a question the passages of the bases of the agent that fit it best are given to the model with the question. The passages are an index, made again from the text, not a model.
- **The journal of the AI** keeps every question and its answer, only ever added to, with the work unit it was asked for: every result of the AI can be traced to what it rests on. The integration, the model and the agent are kept by their names, so that an entry stays as it was.

## 5. The user and the program

- One Linux account holds one installation with one user. `User` and `Installation` are kept in the system database, which is not encrypted and holds nothing secret; the copies of the key of the encrypted database of the user are kept beside the user but belong to `db/encryption/` (`architecture.md`, section 5).
- `Settings` names every setting as a field of its own: today the integration used where no agent names one.
