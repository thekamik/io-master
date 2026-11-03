# Evaluation report: {{evaluation_plan.product}}

| | |
|---|---|
| Product | {{evaluation_plan.product}} |
| Client | {{evaluation_plan.client}} |
| Evaluator | {{evaluation_plan.evaluator}} |
| Begun | {{evaluation_plan.started}} |
| Plan | {{evaluation_plan.source_plan}} |
| State | {{evaluation_plan.state}} |
| Agent | {{evaluation_plan.ai_agent}} |
| Verdicts | {{evaluation_plan.possible_verdicts}} |

## Standards

{{#evaluation_plan.oscal_documents}}- {{oscal_standard_document.title}}, version {{oscal_standard_document.version}}
{{/evaluation_plan.oscal_documents}}
## Documents of the product

{{#evaluation_plan.product_documents}}- {{product_document.name}}, version {{product_document.version}}
{{/evaluation_plan.product_documents}}
## Proofs of the laboratory

{{#evaluation_plan.lab_proofs}}- {{lab_proof.name}}: performed on {{lab_proof.performed}} by {{lab_proof.tester}}, with {{lab_proof.equipment}}
{{/evaluation_plan.lab_proofs}}
## Work units

{{#evaluation_plan.work_units}}### {{work_unit.label}}: {{work_unit.verdict}}

{{work_unit.requirement}}

- Standard: {{work_unit.reference}}
- Documents: {{work_unit.input_product_documents}}
- Proofs: {{work_unit.input_functional}}
- Dependencies: {{work_unit.dependencies}}
- Assessed by: {{work_unit.author}}

**Findings.** {{work_unit.findings}}

**Analysis.** {{work_unit.analysis}}

**Vulnerability notes.** {{work_unit.vulnerability_notes}}

**Notes.** {{work_unit.notes}}

**Quotes.**

{{work_unit.quotes}}
{{work_unit.proof_quotes}}

{{/evaluation_plan.work_units}}
## Observations

{{#evaluation_plan.work_units verdict=fail}}- {{work_unit.label}} (fail): {{work_unit.findings}}
{{/evaluation_plan.work_units}}{{#evaluation_plan.work_units verdict=inconclusive}}- {{work_unit.label}} (inconclusive): {{work_unit.findings}}
{{/evaluation_plan.work_units}}
