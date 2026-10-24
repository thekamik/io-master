# Step 41: the evaluation plan and the evaluation on NIST OSCAL (concept)

**State: approved by the user on 2026-10-18 (sections 1 and 8); built part by part (section 8).**

OSCAL (Open Security Controls Assessment Language, NIST) is the published, machine-readable format for requirements (*controls*) and their assessment. The program adopts it as the format of its data about standards, in the version **1.2.3** (the latest release at the time of writing; the version is pinned and changed only by a decision).

---

## 1. Decisions of the user (2026-10-18)

1. **Two functions of the work with standards:**
   - **a) Evaluation plan:** making or loading the scheme of a standard in OSCAL, showing it as a graph of the activities and requirements of the standard, and handing it over to b).
   - **b) Evaluation:** taking a plan of a) and, with the AI, providing the evidence, checking whether the requirements are met, and making the report.

   This is strong support for the laboratory, not the whole process of certification. Besides it, the program integrates the tools with which the laboratory carries out the functional tests.
2. **The data of the standards is stored in the NIST standard.** What the program needs beyond it (instructions to the AI, prompts, the work of the evaluation) is **kept apart and only linked to it**: two models that complete each other, **the NIST standard** (where the standard is described) and **the evaluation support** (our own standard, linked to the NIST one).
3. The models of OSCAL in use now: catalog, mapping, profile, assessment plan, assessment results; the system security plan and the component definition later, with step 30.
4. **The earlier data is deleted** (no migration of the test projects and evaluations of the test installations). The NIST standard is the priority now; it strengthens the credibility of the program.
5. The tests run on files of NIST, but the program must also **make schemes of its own**: the standards that matter most are **Common Criteria** and **ETSI EN 303 645**, which are not published in OSCAL.
6. **No solution of its own for one standard** (second round): every standard is represented in OSCAL, by the same code; the evaluation plan decides what an evaluation covers, and the evaluation serves every case. The tables of section 3 are examples of how the author of a catalog represents a standard, not code of the program.
7. **Every document that defines the work is a document of OSCAL and can be taken into a plan**: the criteria (CC Parts 1 to 5, ETSI EN 303 645), the methodology with its work units, which define the tests (the CEM), the Protection Profiles, which say directly which tests to carry out, and test specifications (ETSI TS 103 701). A plan may take several of them.
8. The storage of section 8, question 1, way A (the document kept as its JSON, checked against the schema at every write; the types of `model/oscal/` with serde, an exception to the rule of serialization). The order of section 8, question 3; the old model of the test projects and the evaluations is removed: the model of NIST is the better one.

---

## 2. The path of the work

```text
 documents of a standard (PDF, standards/)
        │  AI proposes, the evaluator accepts; every quote checked on its page
        ▼
 CATALOG (OSCAL)         the requirements of the standard: groups, controls, parts,
        │                parameters, assessment objectives; the PDFs as resources
 MAPPING (OSCAL)         relations between requirements, also of two standards
        ▼
 PROFILE (OSCAL)         the requirements chosen for one evaluation, parameters set
        ▼
 ASSESSMENT PLAN (OSCAL) the evaluation plan: tasks, their order, activities, subjects,
        │                tools; shown and edited as a graph       ── function a)
        ▼  handed over
 ASSESSMENT RESULTS (OSCAL)  observations (evidence), findings per objective,
                             risks, the log of the assessment     ── function b)
        ▼
 report (LaTeX template of the laboratory, step 28)

 EVALUATION SUPPORT (own)    linked by the UUIDs and ids of the OSCAL objects:
                             AI instructions and prompts, quotes with their pages and
                             the result of their check, proposals of the AI and their
                             state, the layout of the graph, the journal of the AI
```

## 3. How the standards are represented (examples, not code)

One model for all: catalogs (requirements, work units, test cases), profiles (a selection with its parameters: a Protection Profile, a Security Target), mappings (relations between the documents). The rows below show how the documents named by the user are represented by their authors (the AI proposes, the evaluator accepts).

### 3.1 Common Criteria (ISO/IEC 15408, CEM ISO/IEC 18045)

| CC | OSCAL catalog |
|---|---|
| class (FAU, ADV) | `group` |
| family (FAU_GEN, ADV_FSP) | `group` inside the class |
| component (FAU_GEN.1, ADV_FSP.4) | `control` |
| element (FAU_GEN.1.1; ADV_FSP.4.1D/C/E) | `part` `name="item"` of the `statement` |
| operation `[assignment: ...]`, `[selection: ...]` | `param` (with `select`/`choice` for a selection); filled in the profile |
| dependency (FAU_GEN.1 needs FPT_STM.1) | `link` `rel="required"`, and the mapping |
| CEM work unit (ADV_FSP.4-1) | the CEM is a catalog of its own (decision 7): an activity is a group, a work unit a control with its `assessment-method` (examine, interview, test); a mapping links it to the evaluator action element of CC Part 3 |
| application notes | `part` `name="guidance"` |

The operations of CC (assignment, selection) are exactly what OSCAL calls parameters: the Security Target sets them, the profile records them.

### 3.2 ETSI EN 303 645 (and ETSI TS 103 701, its test specification)

| ETSI | OSCAL |
|---|---|
| section (5.1 No universal default passwords) | `group` |
| provision (Provision 5.1-1) | `control`; `prop` for its status (mandatory, recommendation, conditional) |
| conditions of a conditional provision | `part` with the condition, `param` where it is chosen |
| test cases of TS 103 701 | `assessment-objective` / `assessment-method` parts, or a catalog of their own with a mapping to EN 303 645 |

### 3.3 Catalogs of NIST

Catalogs published in OSCAL (SP 800-53, ...) are imported as they are; they are the test data of the import and of the checks against the schema.

## 4. The evaluation plan (function a)

- **Profile:** the controls chosen for one evaluation (for CC: the SFRs and SARs of the Security Target, the package of assurance) and the parameters set.
- **Assessment plan:** the tasks (`task`, with `dependencies`, the "precedes" of today), the activities (`activity` with `steps` and `related-controls`), the subjects (`assessment-subjects`), the tools of the functional tests (`assessment-assets`, `assessment-platforms`), the reviewed controls and objectives.
- **The graph** is a view of these objects: requirements (controls and their objectives), activities and tasks, their links; its layout (the place of a node) is evaluation support.
- **The import of a system security plan** (`import-ssp`), which OSCAL requires in an assessment plan: until step 30, it points to a resource of the back matter (the Security Target of the product, a PDF) instead of a full SSP. *To be confirmed against the schema.*

## 5. The evaluation (function b)

- **Assessment results:** the observations (`observation`: method examine / interview / test, the evidence in `relevant-evidence`), the findings per objective (`finding` with `target` and its status), the risks (vulnerabilities), and the **log of the assessment** (`assessment-log`): the journal of today.
- **The decision stays with the evaluator (P1):** what the AI proposes (fragments, an assessment) is evaluation support until the evaluator accepts it; only then it becomes an observation or a finding of the results.
- **The four text results of a requirement** (findings; analysis; assessment and verdict proposition; vulnerabilities and notes, user 2026-10-17) map onto the finding (`description`, `remarks`), its observations and its risks; **the status** of a finding (satisfied / not satisfied) is set by the evaluator. *To be checked: the verdict "inconclusive" of CC, which OSCAL has no status for (a property).*
- **Functional tests:** the tools are assessment assets of the plan; the result of a test the laboratory carried out, delivered by the evaluator, becomes an observation with the method test (the program does not run tests on a device, step 48).

## 6. The evaluation support (own standard)

Tables of their own, in `db/evaluation_support/`, linked to OSCAL objects only by their UUID or id (`catalog uuid + control id`, `part id`, `task uuid`, ...); nothing of ours is written into an OSCAL document. An export gives the OSCAL files (valid against the schema of NIST) and, apart, a file of the evaluation support that names the same UUIDs.

| Holds | Linked to |
|---|---|
| the quotes a control or part comes from: document, page, quote, found | control, part |
| the AI notes (never presented as quotes) | control, part, observation |
| proposals of the AI and their state (proposed, accepted, rejected) | any object |
| instructions to the AI and prompts of the tasks | catalog, plan |
| the layout of the graph | node of the plan |
| the journal of the AI | stays in `db/ai/journal/`, linked by the UUIDs it concerns |

## 7. The code

- `model/oscal/`: the types of the subset of OSCAL we use (catalog, mapping, profile, assessment plan, assessment results, the common metadata and back matter), with the names of OSCAL.
- `db/oscal/` (one folder per model) and `db/evaluation_support/`.
- The view Converter (`views/converter/`, with its editor of a catalog `views/converter/catalog_editor/`, step 50), the view Evaluation plan (`views/evaluation_plan/`, only the documents of OSCAL) and, later, of the evaluation on the new model (step 42 replaced `business/` by the views); `ai/tasks/` builds catalogs from the PDFs of a standard.
- **The schema:** the JSON schemas of OSCAL 1.2.3 kept in the repository; every import and every export is checked against them (a new dependency, the crate `jsonschema`, explained when it is added).
- The old model of the test projects and the evaluations is removed, with its tables (no migration).

## 8. Open questions for the user

1. **How an OSCAL document is stored and how it becomes JSON.** OSCAL is large. Two ways:
   - **A (proposed):** the database keeps each OSCAL document as its JSON, checked against the schema at every write, with tables of index (ids, UUIDs) for the links and the queries; the types of `model/oscal/` are turned into the JSON of OSCAL by serde, with the names of OSCAL. This is an **exception to the rule "the core types carry no serialization code"**: these types *are* a published format, and a hand-written conversion of hundreds of fields would be a large source of errors.
   - **B:** normalized tables per OSCAL object, and a conversion written by hand (as the file of a test project today): no exception to the rule, but much more code to review.
2. **The CEM work units:** as assessment objectives inside the controls of CC Part 3 (proposed: what OSCAL objectives are for), or a catalog of the CEM of its own with a mapping.
3. **The order of the parts:** 41.1 the rename and the removal of the old model; 41.2 the foundation (schemas, `model/oscal/`, `db/oscal/`, import and export of NIST catalogs); 41.3 own catalogs (CC, ETSI EN 303 645) made by the AI from the PDFs and edited by the evaluator, with the evaluation support; 41.4 profile and assessment plan with the graph; 41.5 assessment results, then steps 31 to 36 on them.

### Answers of the user (2026-10-18)

1. Way A.
2. The CEM is a document of its own in OSCAL (it defines the tests through its work units), and so are the Protection Profiles (which say directly which tests to carry out); each can be taken into a plan.
3. The order 41.1 to 41.5 as proposed.
