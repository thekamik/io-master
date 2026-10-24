# NIST OSCAL in IO Master

**State (2026-10-24):** the catalogs of OSCAL are the working model of the requirements; the plan of an evaluation and the evaluation itself are the program's own model, linked to the catalogs by their UUIDs and ids. The concept of step 41 (2026-10-18) is recorded in section 5; its parts 41.4 and 41.5 were replaced by steps 51 and 52 (`history.md`).

OSCAL (Open Security Controls Assessment Language, NIST) is the published, machine-readable format for requirements (*controls*) and their assessment. The program adopts it in the version **1.2.3** (pinned; changed only by a decision). Every document of OSCAL is checked against the JSON schemas of NIST (`io-master/oscal/`) whenever it is written.

---

## 1. What the program does with OSCAL

```text
 PDFs of a standard ──Converter (AI proposes, the evaluator accepts;
        │             every quote checked on its page)
        ▼
 CATALOG (OSCAL)  ◄── imported: catalogs published in OSCAL (NIST SP 800-53),
        │             the CEM (testing_documents/), the Protection Profiles, ...
        │  linked by the UUID of the catalog and the ids of its controls
        ▼
 MASTER PROJECT (own model, Evaluation plan): tasks = whole requirements,
        │        the scheme of each task, related elements, work packages
        ▼  work package: our format + the catalogs attached, whole and unchanged
 EVALUATION (own model, Evaluation lab): evidence, fragments, four results,
        │        a verdict, the texts of the outputs
        ▼
 REPORT: one file per output (.txt, .tex), for the template of the laboratory

 MAPPING, PROFILE, ASSESSMENT PLAN, ASSESSMENT RESULTS (OSCAL): imported, checked,
        kept and exported (Evaluation plan, tab OSCAL documents); not yet made by the program
```

- **Catalogs** are made in the Converter from the PDFs of a standard (step 50), or imported. A catalog made in the program names its PDFs as resources of its back matter, by the SHA-256 of the file; its controls carry the identifiers of the standard (`prop` `label`). A requirement of the laboratory, outside the standards, is a control of one shared catalog "Requirements of the laboratory" (step 51.1e).
- **The other models** (mapping, profile, assessment plan, assessment results) are documents the program takes in and gives out unchanged, so that a laboratory can keep them beside its work.
- **The plan and the evaluation are our own model** (steps 51 and 52), because OSCAL has no place for what the work needs: the scheme of a task (the evidence expected at its inputs, the instructions of the agents of the AI, the outputs with their conditions), the work packages delegated to the evaluators, the raw evidence with its fragments. They point to OSCAL only by the UUID of a catalog and the id of a control; **nothing of ours is written into a document of OSCAL**, so every export of OSCAL is valid against the schema of NIST.
- **An export of the plan and of the results as OSCAL** (a master project as a profile and an assessment plan; an evaluation as assessment results, the four results as a finding with its observations and risks) is a reminder of `todo.md`, built when a laboratory or a scheme asks for it. The mapping planned in section 5 below holds for it.

## 2. The evaluation support (own model)

Tables of their own, linked to OSCAL objects only by their UUID or id (`catalog uuid + control id`):

| Holds | Where | Linked to |
|---|---|---|
| the quotes a group or a control comes from: document, page, quote, found | `db/evaluation_support/` | group, control |
| the proposals of the AI for a catalog and their state | `db/evaluation_support/` | catalog |
| the conversion of a catalog: its stage, the pages of its groups | `db/conversion/` | catalog |
| the master projects: catalogs, tasks, schemes, proposals, related elements, work packages | `db/evaluation_project/` | catalog, control |
| the evaluations: the frozen copy of a package, evidence, fragments, results, outputs | `db/evaluation/` | catalog, control |
| the instructions to the AI of every task | `db/ai/instructions/` | — |
| the journal of the AI | `db/ai/journal/` | — |

## 3. The code

- `model/oscal/`: the types of the subset of OSCAL we use (catalog, mapping, profile, assessment plan, assessment results, the common metadata and back matter), with the names of OSCAL; they carry serde, **the one exception to the rule of serialization** (decision 8 below): these types *are* the published format.
- `db/oscal/`: every document kept as its JSON, checked against the schema at every write, with tables of index for its UUIDs and ids.
- The view Converter (`views/converter/`) makes catalogs; the view Evaluation plan (`views/evaluation_plan/`) imports, exports and deletes the documents and uses the catalogs in the master projects; the view Evaluation lab (`views/evaluation_lab/`) imports the catalogs attached to a work package.
- `model/oscal/schema.rs`: the check against the JSON schemas of NIST (the crate `jsonschema`).

---

## 4. How the standards are represented (examples, not code)

One model for all: catalogs (requirements, work units, test cases), profiles (a selection with its parameters: a Protection Profile, a Security Target), mappings (relations between the documents). The rows below show how the documents named by the user are represented by their authors (the AI proposes, the evaluator accepts).

### 4.1 Common Criteria (ISO/IEC 15408, CEM ISO/IEC 18045)

| CC | OSCAL catalog |
|---|---|
| class (FAU, ADV) | `group` |
| family (FAU_GEN, ADV_FSP) | `group` inside the class |
| component (FAU_GEN.1, ADV_FSP.4) | `control` |
| element (FAU_GEN.1.1; ADV_FSP.4.1D/C/E) | `part` `name="item"` of the `statement` |
| operation `[assignment: ...]`, `[selection: ...]` | `param` (with `select`/`choice` for a selection); filled in the profile |
| dependency (FAU_GEN.1 needs FPT_STM.1) | `link` `rel="required"`, and the mapping |
| CEM work unit (ADV_FSP.4-1) | the CEM is a catalog of its own (decision 7 below): an activity is a group, a work unit a control with its `assessment-method` (examine, interview, test); a mapping links it to the evaluator action element of CC Part 3 |
| application notes | `part` `name="guidance"` |

The operations of CC (assignment, selection) are exactly what OSCAL calls parameters: the Security Target sets them, the profile records them.

### 4.2 ETSI EN 303 645 (and ETSI TS 103 701, its test specification)

| ETSI | OSCAL |
|---|---|
| section (5.1 No universal default passwords) | `group` |
| provision (Provision 5.1-1) | `control`; `prop` for its status (mandatory, recommendation, conditional) |
| conditions of a conditional provision | `part` with the condition, `param` where it is chosen |
| test cases of TS 103 701 | `assessment-objective` / `assessment-method` parts, or a catalog of their own with a mapping to EN 303 645 |

### 4.3 Catalogs of NIST

Catalogs published in OSCAL (SP 800-53, ...) are imported as they are; they are the test data of the import and of the checks against the schema.

## 5. The concept of step 41 (2026-10-18), for the record

The user's decisions of 2026-10-18 set OSCAL as the format of the standards. The path planned then went catalog → profile → assessment plan (a graph) → assessment results → report. The catalogs were built as planned (41.1 to 41.3); the plan and the evaluation were built instead as steps 51 and 52 on the program's own model (section 1). The mapping planned for the results stays valid for a later export: the observations (`observation`: method examine / interview / test, the evidence in `relevant-evidence`), the findings per objective (`finding` with `target` and its status), the risks (vulnerabilities), the log of the assessment; the four text results onto the finding, its observations and its risks; the verdict "inconclusive", which OSCAL has no status for, as a property.

### 5.1 Decisions of the user (2026-10-18)

1. **Two functions of the work with standards:**
   - **a) Evaluation plan:** making or loading the scheme of a standard in OSCAL, showing it as a graph of the activities and requirements of the standard, and handing it over to b).
   - **b) Evaluation:** taking a plan of a) and, with the AI, providing the evidence, checking whether the requirements are met, and making the report.

   This is strong support for the laboratory, not the whole process of certification. Besides it, the program integrates the tools with which the laboratory carries out the functional tests.
2. **The data of the standards is stored in the NIST standard.** What the program needs beyond it (instructions to the AI, prompts, the work of the evaluation) is **kept apart and only linked to it**: two models that complete each other, **the NIST standard** (where the standard is described) and **the evaluation support** (our own standard, linked to the NIST one).
3. The models of OSCAL in use now: catalog, mapping, profile, assessment plan, assessment results; the system security plan and the component definition later, with step 30.
4. **The earlier data is deleted** (no migration of the test projects and evaluations of the test installations). The NIST standard is the priority now; it strengthens the credibility of the program.
5. The tests run on files of NIST, but the program must also **make schemes of its own**: the standards that matter most are **Common Criteria** and **ETSI EN 303 645**, which are not published in OSCAL.
6. **No solution of its own for one standard** (second round): every standard is represented in OSCAL, by the same code; the evaluation plan decides what an evaluation covers, and the evaluation serves every case. The tables of section 4 are examples of how the author of a catalog represents a standard, not code of the program.
7. **Every document that defines the work is a document of OSCAL and can be taken into a plan**: the criteria (CC Parts 1 to 5, ETSI EN 303 645), the methodology with its work units, which define the tests (the CEM), the Protection Profiles, which say directly which tests to carry out, and test specifications (ETSI TS 103 701). A plan may take several of them.
8. The storage of section 5.2, question 1, way A (the document kept as its JSON, checked against the schema at every write; the types of `model/oscal/` with serde, an exception to the rule of serialization). The order of section 5.2, question 3; the old model of the test projects and the evaluations is removed: the model of NIST is the better one.

---

### 5.2 The questions of step 41 and the answers of the user

1. **How an OSCAL document is stored and how it becomes JSON.** OSCAL is large. Two ways:
   - **A (proposed):** the database keeps each OSCAL document as its JSON, checked against the schema at every write, with tables of index (ids, UUIDs) for the links and the queries; the types of `model/oscal/` are turned into the JSON of OSCAL by serde, with the names of OSCAL. This is an **exception to the rule "the core types carry no serialization code"**: these types *are* a published format, and a hand-written conversion of hundreds of fields would be a large source of errors.
   - **B:** normalized tables per OSCAL object, and a conversion written by hand (as the file of a test project today): no exception to the rule, but much more code to review.
2. **The CEM work units:** as assessment objectives inside the controls of CC Part 3 (proposed: what OSCAL objectives are for), or a catalog of the CEM of its own with a mapping.
3. **The order of the parts:** 41.1 the rename and the removal of the old model; 41.2 the foundation (schemas, `model/oscal/`, `db/oscal/`, import and export of NIST catalogs); 41.3 own catalogs (CC, ETSI EN 303 645) made by the AI from the PDFs and edited by the evaluator, with the evaluation support; 41.4 profile and assessment plan with the graph; 41.5 assessment results, then steps 31 to 36 on them.

**Answers of the user (2026-10-18)**

1. Way A.
2. The CEM is a document of its own in OSCAL (it defines the tests through its work units), and so are the Protection Profiles (which say directly which tests to carry out); each can be taken into a plan.
3. The order 41.1 to 41.5 as proposed.
