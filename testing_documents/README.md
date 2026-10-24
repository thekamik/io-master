# Testing documents

Documents for trying the program by hand. They are not used by the automatic tests (those use the files of NIST in `io-master/oscal/examples/`).

## CEM2022R1_work_units_catalog.json

The **work units of the CEM:2022 Revision 1** (Common Methodology for Information Technology Security Evaluation, CCMB-2022-11-006, the methodology of Common Criteria, ISO/IEC 18045) as an **OSCAL 1.2.3 catalog**. It is imported in the window (Evaluation plan → OSCAL documents → Import an OSCAL document…), opened in the editor of catalogs (Converter), and taken into a master project as the catalog of the methodology, whose work units the AI links to the requirements of CC Part 3 as related elements (step 51.4). It is checked against the schema of NIST and imported, exported and opened by the program without a change.

Source: <https://www.commoncriteriaportal.org/files/ccfiles/CEM2022R1.pdf> (471 pages), SHA-256 `da5b991a457eb32040a15849bfe6d8da1870aef91f6b7255f767f0b186e89a83`. The catalog names this file in its back matter by the same SHA-256, as the catalogs made in the program do.

**The structure** follows the CEM (`docs/oscal.md`, section 3.1: the CEM is a catalog of its own):

```text
group    class         APE, ACE, ASE, ADV, AGD, ALC, ATE, AVA, ACO
 group   family        ADV_ARC
  group  sub-activity  ADV_ARC.1    parts: objective, input (the evaluation evidence), application-notes
   group action        ADV_ARC.1.1E
    control work unit  ADV_ARC.1-1
```

A work unit (control) has:

- **title**: the first sentence of the work unit without "The evaluator shall" (shortened on a word);
- **statement** (part): the text of the work unit;
- **cc-element** (parts and props): the elements of CC Part 3 the work unit checks (`ADV_ARC.1.1C`), with their text;
- **assessment-method** (part, prop `method`): **TEST** where the laboratory carries out tests on the TOE (it devises the tests, writes their documentation, runs them, records their results, carries out the procedures of the guidance on the TOE), else **EXAMINE** (examining, checking, analysing the evidence, reporting);
- **guidance** (part): the explanations of the CEM that follow the work unit;
- props `label` (the number in the CEM), `action-element` (`ADV_ARC.1.1E`) and `verb` (the verb of "shall ...": examine, check, conduct, ...).

Our own names (props and parts outside the names of NIST) carry the namespace `https://github.com/thekamik/io-master/ns/cem`.

**Not in the catalog:** chapter 9 of the CEM (the evaluation process, the tasks of input and output: the observation report and the ETR), which has no work units, and the annexes (general evaluation guidance).

**Two slips of the CEM text, kept as they are:** a work unit of ACE_OBJ.1 is numbered `APE_OBJ.1-3` in the CEM; in the catalog its id is `ACE_OBJ.1-3` (OSCAL ids must be unique) and its prop `label` keeps `APE_OBJ.1-3`. The numbering of ACE_OBJ.2 skips `ACE_OBJ.2-5`, as in the CEM.

**Made from the text of the PDF by a script** (`tools/`): the text is read with its positions on the page (`pdftohtml -xml` of Poppler), paragraphs are told by the gaps between lines, the heads and feet of the pages and the footnotes are left out. Check a work unit against the PDF before it is used in an evaluation. To make the file again:

```bash
pdftohtml -xml -i -q CEM2022R1.pdf cem
python3 -I tools/cemxml2text.py cem.xml cem-para.txt
python3 -I tools/cem2oscal.py cem-para.txt CEM2022R1.pdf CEM2022R1_work_units_catalog.json
```

The UUIDs are derived from the names (UUID version 5), so the same PDF gives the same UUIDs; only `last-modified` changes.

**Licence.** The text of the CEM in `CEM2022R1_work_units_catalog.json` is **not covered by the licence of the project (GPL-2, `LICENSE`)**; it is reused, in a changed form (the structure of OSCAL, the titles, the methods TEST and EXAMINE), under the legal notice of the CEM:2022 (page vii of the PDF):

> The governmental organizations listed below contributed to the development of this version of the Common Methodology for Information Technology Security Evaluation. As the joint holders, together with ISO/IEC, of the copyright in the Common Methodology for Information Technology Security Evaluation, version 2022 (called "CEM:2022"), they hereby grant a non-exclusive permission to ISO/IEC to reproduce CEM:2022 in the revised editions of ISO/IEC 18045 and its derivatives, including their national adoptions. However, these governmental organizations retain the right to use, copy, distribute, translate or modify CEM:2022 as they see fit. ISO/IEC has in return granted permission to the aforementioned organizations to license the resulting CEM:2022 under any licence they may see appropriate. The aforementioned governmental organizations have always been supportive of the text being reused by any users of the document, including modifications and reuse of part of the document, and will continue to follow this policy.

The copyright of the text stays with its holders; the source is named above. The scripts in `tools/` are part of the project and are under its licence (GPL-2).

### Work units per class

| Class | | Work units | TEST |
|---|---|---:|---:|
| APE | Protection Profile evaluation | 77 | 0 |
| ACE | Protection Profile Configuration evaluation | 120 | 0 |
| ASE | Security Target evaluation | 108 | 0 |
| ADV | Development | 162 | 0 |
| AGD | Guidance documents | 11 | 1 |
| ALC | Life-cycle support | 140 | 0 |
| ATE | Tests | 63 | 9 |
| AVA | Vulnerability assessment | 60 | 21 |
| ACO | Composition | 72 | 8 |
| | **All** | **813** | **39** |

### The tests the laboratory carries out (method TEST)

| Work unit | Text |
|---|---|
| AGD_PRE.1-3 | The evaluator shall perform all user procedures necessary to prepare the TOE to determine that the TOE and its operational environment can be prepared securely using only the supplied preparative procedures. |
| ATE_IND.1-3 | The evaluator shall devise a test subset. |
| ATE_IND.1-4 | The evaluator shall produce test documentation for the test subset that is sufficiently detailed to enable the tests to be reproducible. |
| ATE_IND.1-5 | The evaluator shall conduct testing. |
| ATE_IND.1-6 | The evaluator shall record the following information about the tests that compose the test subset: |
| ATE_IND.2-4 | The evaluator shall conduct testing using a sample of tests found in the developer test plan and procedures. |
| ATE_IND.2-6 | The evaluator shall devise a test subset. |
| ATE_IND.2-7 | The evaluator shall produce test documentation for the test subset that is sufficiently detailed to enable the tests to be reproducible. |
| ATE_IND.2-8 | The evaluator shall conduct testing. |
| ATE_IND.2-9 | The evaluator shall record the following information about the tests that compose the test subset: |
| AVA_VAN.1-5 | The evaluator shall devise penetration tests, based on the independent search for potential vulnerabilities. |
| AVA_VAN.1-6 | The evaluator shall produce penetration test documentation for the tests based on the list of potential vulnerabilities in sufficient detail to enable the tests to be repeatable. The test documentation shall include: |
| AVA_VAN.1-7 | The evaluator shall conduct penetration testing. |
| AVA_VAN.1-8 | The evaluator shall record the actual results of the penetration tests. |
| AVA_VAN.2-6 | The evaluator shall devise penetration tests, based on the independent search for potential vulnerabilities. |
| AVA_VAN.2-7 | The evaluator shall produce penetration test documentation for the tests based on the list of potential vulnerabilities in sufficient detail to enable the tests to be repeatable. The test documentation shall include: |
| AVA_VAN.2-8 | The evaluator shall conduct penetration testing. |
| AVA_VAN.2-9 | The evaluator shall record the actual results of the penetration tests. |
| AVA_VAN.3-6 | The evaluator shall devise penetration tests, based on the independent search for potential vulnerabilities. |
| AVA_VAN.3-7 | The evaluator shall produce penetration test documentation for the tests based on the list of potential vulnerabilities in sufficient detail to enable the tests to be repeatable. The test documentation shall include: |
| AVA_VAN.3-8 | The evaluator shall conduct penetration testing. |
| AVA_VAN.3-9 | The evaluator shall record the actual results of the penetration tests. |
| AVA_VAN.4-6 | The evaluator shall devise penetration tests, based on the independent search for potential vulnerabilities. |
| AVA_VAN.4-7 | The evaluator shall produce penetration test documentation for the tests based on the list of potential vulnerabilities in sufficient detail to enable the tests to be repeatable. The test documentation shall include: |
| AVA_VAN.4-8 | The evaluator shall conduct penetration testing. |
| AVA_VAN.4-9 | The evaluator shall record the actual results of the penetration tests. |
| AVA_VAN.5-6 | The evaluator shall devise penetration tests, based on the independent search for potential vulnerabilities. |
| AVA_VAN.5-7 | The evaluator shall produce penetration test documentation for the tests based on the list of potential vulnerabilities in sufficient detail to enable the tests to be repeatable. The test documentation shall include: |
| AVA_VAN.5-8 | The evaluator shall conduct penetration testing. |
| AVA_VAN.5-9 | The evaluator shall record the actual results of the penetration tests. |
| AVA_COMP.1-2 | The evaluator shall specify, conduct and document penetration testing of the composite product as a whole, using the standard approach of the assurance family AVA_VAN. |
| ACO_CTT.1-7 | The evaluator shall perform testing in accordance with ATE_IND.2.2E, for a subset of the SFRs specified in the composed security target, to verify the developer test results. |
| ACO_CTT.1-8 | The evaluator shall perform testing in accordance with ATE_IND.2.3E, for a subset of the SFRs specified in the composed security target, to confirm that the TSF operates as specified. |
| ACO_CTT.2-9 | The tests are to be selected and executed in accordance with ATE_IND.2.2E, to demonstrate the correct behaviour of the SFRs specified in the composed TOE security target. |
| ACO_CTT.2-10 | The evaluator shall perform testing in accordance with ATE_IND.2.3E, for a subset of the SFRs specified in the composed security target, to confirm that the TSF operates as specified. |
| ACO_CTT.2-11 | The evaluator shall perform testing, in accordance with Evaluation of sub-activity (ATE_IND.2), for a subset of the interfaces to the base component to confirm they operate as specified. |
| ACO_VUL.1-8 | The evaluator shall conduct penetration testing as detailed for AVA_VAN.1.3E. |
| ACO_VUL.2-9 | The evaluator shall conduct penetration testing as detailed for AVA_VAN.2.4E. |
| ACO_VUL.3-9 | The evaluator shall conduct penetration testing as detailed for AVA_VAN.3.4E. |
