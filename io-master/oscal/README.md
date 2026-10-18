# OSCAL files of NIST

Files of NIST, kept unchanged in the repository (works of the US government, in the public domain), used by the program (step 41 of `docs/todo.md`, `docs/oscal.md`):

- `schema/v1.2.3/`: the JSON schemas of the OSCAL models the program uses, from the release v1.2.3 of NIST (published 2026-08-07): <https://github.com/usnistgov/OSCAL/releases/tag/v1.2.3>. Every OSCAL document the program stores or exports is checked against them (`src/model/oscal/schema.rs`). The version is changed only by a decision.
- `examples/`: example documents of NIST (<https://github.com/usnistgov/oscal-content/tree/main/examples>), the test data of the import, the export and the checks.

Fetched on 2026-10-18; their SHA-256:

```text
ea687b9d0ab1d84c9cb11ee0a5e22b17956fe892ee93f5acca937bef81d23ea2  oscal/schema/v1.2.3/oscal_assessment-plan_schema.json
4034e2032332dbf597e59e0646ec16c2c31df992962490371e91f47b219ff42c  oscal/schema/v1.2.3/oscal_assessment-results_schema.json
ab95836e9e8dfeb6fde80007f6cc76fa3192f595d427c751a3f3923c3f474fc2  oscal/schema/v1.2.3/oscal_catalog_schema.json
ed28b68c8e56cc7d39f0c6076a5d7da3e8100fbe25abf5bb731120bd34f8817e  oscal/schema/v1.2.3/oscal_mapping_schema.json
7c5ff5a92683b6a80ce6a3474dea04e0ea8680a4ecb60702ffa08ebfc677e6cb  oscal/schema/v1.2.3/oscal_profile_schema.json
e6c8c0b0617686dd563a0926a11fea2dadeb7ac9433328d3d02cced1ca1a2ed4  oscal/examples/basic-catalog.json
23f1544794b4190e468caf5883f46eeeef9819c771d4bda713a1b5c3b93fcb9f  oscal/examples/basic-profile.json
30b3c50c5c40bae44fa306b4ad4dfe26c36b1b464cab1846c84bce9656cc4988  oscal/examples/ifa_assessment-plan-example.json
13bde5421ac9f8986e293b74e7caa025fd1f61a4c55d94bb68dce44417ff0eb6  oscal/examples/ifa_assessment-results-example.json
```
