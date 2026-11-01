# IO Master: the map of the architecture

Where each model of `io-master/src/model/` and `io-master/src/objects/` is stored, processed, reached by a command of the page and shown (user, 2026-11-01): a list, model by model, so that every flow of the data can be followed from the model to the window and nothing is missed. It changes in the same commit as the code; a graphical map may come later from it.

```text
model/  ──► db/models/ (stored) ──► domain/models/, ai/models/, session/, panels/logic/ (processed)
        ──► panels/views/ (the commands of the page) ──► ui/src/ (the components that show it)
```

How it is read:

- **Stored**: the functions of `db/models/`, or of the store of the object (`<object>_store.rs`), that create, read and update the model.
- **Processed**: the functions that call them or take the model itself, by concern: the rules of the data (`domain/`, or the rules of an object, `<object>_rules.rs`), the AI (`ai/`), the session, and the panels not yet brought to the grid (`panels/logic/`).
- **Commands**: the commands of `panels/views/` that reach the model within three calls.
- **Shown**: the components of the page that call those commands (`ui/src/lib/api/` names each function as its command).

A function of `panels/views/` that only prepares a view, and the tests, are left out.

## The models

### `objects/ai/agent/agent_model.rs`: `Agent`, `GlossaryEntry`

- **Stored** (`objects/ai/agent/agent_store.rs`): `create_agent`, `read_agent`, `read_agents`, `update_agent`
- **Processed, the rules of the data**:
  - `objects/ai/agent/agent_rules.rs`: `change_agent`, `checked`, `checked_glossary`, `new_agent`
  - `objects/ai/chat/chat_rules.rs`: `new_chat`
  - `domain/models/evaluation/evaluation_plan/evaluation.rs`: `change_agent`, `new_evaluation`
  - `domain/models/evaluation/evaluation_plan/planning.rs`: `update_plan`
  - `domain/models/evaluation/work_unit/parameters.rs`: `update_work_unit_parameters`
- **Processed, the AI**:
  - `objects/ai/agent/agent_ai.rs`: `agent_knowledge`, `agent_question`, `widened`
  - `ai/models/evaluation/evaluation_plan.rs`: `fill_preview`, `preview`
  - `ai/models/evaluation/work_unit/proposal.rs`: `names_of`
- **Commands** (`panels/views/`):
  - `agents/agent.rs`: `agent_list`, `agent_create`, `agent_get`, `agent_save`, `agent_knowledge_test`
  - `chat/chats.rs`: `chat_list`, `chat_create`, `chat_get`
  - `evaluation/evaluations.rs`: `evaluation_create`, `evaluation_get`, `evaluation_set_agent`
  - `evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`
  - `evaluation/work_units.rs`: `evaluation_unit_list`, `evaluation_unit_get`
  - `evaluation_plan/fill.rs`: `plan_fill_preview`
  - `evaluation_plan/plans.rs`: `plan_get`, `plan_save`
  - `evaluation_plan/work_units.rs`: `work_unit_get`, `work_unit_save`
- **Shown** (`ui/src/`):
  - `views/agents/AgentEditor.svelte` ← `agent_get`, `agent_save`, `agent_knowledge_test`
  - `views/agents/AgentList.svelte` ← `agent_list`, `agent_create`
  - `views/chat/ChatList.svelte` ← `agent_list`, `chat_list`, `chat_create`
  - `views/chat/Conversation.svelte` ← `chat_get`
  - `views/evaluation/EvaluationInputs.svelte` ← `evaluation_get`, `evaluation_set_agent`
  - `views/evaluation/EvaluationRun.svelte` ← `evaluation_assess_preview`, `evaluation_assess`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`
  - `views/evaluation/EvaluationUnits.svelte` ← `evaluation_unit_list`
  - `views/evaluation/Evaluations.svelte` ← `evaluation_create`, `agent_list`
  - `views/evaluation_plan/PlanEditor.svelte` ← `plan_get`
  - `views/evaluation_plan/PlanFill.svelte` ← `plan_fill_preview`
  - `views/evaluation_plan/PlanForm.svelte` ← `plan_get`, `plan_save`
  - `views/evaluation_plan/PlanWorkUnits.svelte` ← `plan_get`
  - `views/evaluation_plan/WorkUnitEditor.svelte` ← `plan_get`, `work_unit_get`, `work_unit_save`

### `objects/ai/api_key/api_key_model.rs`: `ApiKey`

- **Stored** (`objects/ai/api_key/api_key_store.rs`): `create_api_key`, `read_api_key`, `update_api_key`
- **Processed, the AI**:
  - `objects/ai/integration/integration_ai/question.rs`: `prepare`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/settings/integrations.rs`: `has_key`, `set_key`
- **Commands** (`panels/views/`):
  - `settings/integrations.rs`: `integration_list`, `integration_set_key`, `integration_check`
- **Shown** (`ui/src/`):
  - `views/agents/AgentList.svelte` ← `integration_list`
  - `views/settings/IntegrationForm.svelte` ← `integration_set_key`, `integration_check`
  - `views/settings/IntegrationList.svelte` ← `integration_list`

### `objects/ai/chat/chat_model.rs`: `Chat`, `Message`, `Speaker`

- **Stored** (`objects/ai/chat/chat_store.rs`): `create_chat`, `create_message`, `read_chat`, `read_chats`, `update_chat`
- **Processed, the rules of the data**:
  - `objects/ai/chat/chat_rules.rs`: `add_question`, `new_chat`, `rename_chat`
- **Processed, the AI**:
  - `objects/ai/agent/agent_ai.rs`: `agent_question`
  - `objects/ai/chat/chat_ai.rs`: `add_answer`, `chat_question`, `leave_oldest_out`, `search_text`
  - `objects/ai/integration/integration_ai/anthropic.rs`: `body`
  - `objects/ai/integration/integration_ai/openai.rs`: `body`
  - `objects/ai/integration/integration_ai/question.rs`: `prepare`
  - `objects/ai/integration/integration_ai/turns.rs`: `turns`
  - `ai/models/evaluation/work_unit/assessment_question.rs`: `question_of`
  - `ai/models/evaluation/work_unit/proposal.rs`: `question_of`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/settings/integrations.rs`: `check_question`
- **Commands** (`panels/views/`):
  - `chat/chats.rs`: `chat_list`, `chat_create`, `chat_get`, `chat_rename`, `chat_ask`
  - `settings/integrations.rs`: `integration_check`
- **Shown** (`ui/src/`):
  - `views/chat/ChatList.svelte` ← `chat_list`, `chat_create`
  - `views/chat/Conversation.svelte` ← `chat_get`, `chat_rename`, `chat_ask`
  - `views/settings/IntegrationForm.svelte` ← `integration_check`

### `objects/ai/integration/integration_model.rs`: `Integration`, `Protocol`

- **Stored** (`objects/ai/integration/integration_store.rs`): `create_integration`, `read_integration`, `read_integrations`, `update_integration`
- **Processed, the rules of the data**:
  - `objects/ai/agent/agent_rules.rs`: `checked`, `new_agent`
- **Processed, the AI**:
  - `objects/ai/integration/integration_ai/connection.rs`: `read_stream`
  - `objects/ai/integration/integration_ai/protocol.rs`: `body`, `key_headers`, `needs_key`, `read_event`
  - `objects/ai/integration/integration_ai/question.rs`: `answer_tokens`, `check_size`, `prepare`
  - `ai/models/evaluation/evaluation_plan.rs`: `fill_preview`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/settings/integrations.rs`: `activate_integration`, `add_integration`, `change_integration`, `checked`, `set_key`
- **Commands** (`panels/views/`):
  - `agents/agent.rs`: `agent_list`, `agent_create`, `agent_get`, `agent_save`
  - `evaluation_plan/fill.rs`: `plan_fill_preview`
  - `settings/integrations.rs`: `integration_list`, `integration_create`, `integration_save`, `integration_activate`, `integration_set_key`, `integration_check`
- **Shown** (`ui/src/`):
  - `views/agents/AgentEditor.svelte` ← `agent_get`, `agent_save`
  - `views/agents/AgentList.svelte` ← `integration_list`, `agent_list`, `agent_create`
  - `views/chat/ChatList.svelte` ← `agent_list`
  - `views/evaluation/Evaluations.svelte` ← `agent_list`
  - `views/evaluation_plan/PlanFill.svelte` ← `plan_fill_preview`
  - `views/settings/IntegrationForm.svelte` ← `integration_create`, `integration_save`, `integration_set_key`, `integration_check`
  - `views/settings/IntegrationList.svelte` ← `integration_list`, `integration_activate`

### `objects/ai/journal/journal_model.rs`: `JournalEntry`, `Ending`

- **Stored** (`objects/ai/journal/journal_store.rs`): `create_journal_entry`, `read_journal_entry`, `read_journal_entries`, `read_latest_journal_entry`
- **Processed, the AI**:
  - `objects/ai/chat/chat_ai.rs`: `kept_text`
  - `objects/ai/integration/integration_ai/anthropic.rs`: `ending_of`
  - `objects/ai/integration/integration_ai/openai.rs`: `ending_of`
  - `objects/ai/journal/journal_ai.rs`: `record`, `write_entry`
- **Commands** (`panels/views/`):
  - `agents/journal.rs`: `agent_journal_list`, `agent_journal_get`
  - `chat/chats.rs`: `chat_ask`
  - `evaluation/work_units.rs`: `evaluation_unit_get`
  - `settings/integrations.rs`: `integration_check`
- **Shown** (`ui/src/`):
  - `views/agents/AgentJournal.svelte` ← `agent_journal_list`, `agent_journal_get`
  - `views/chat/Conversation.svelte` ← `chat_ask`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`
  - `views/settings/IntegrationForm.svelte` ← `integration_check`

### `objects/ai/rag/rag_model.rs`: `KnowledgeBase`, `Source`

- **Stored** (`objects/ai/rag/rag_store.rs`): `create_knowledge_base`, `read_knowledge_base`, `read_knowledge_bases`, `update_knowledge_base`, `create_source`, `read_source`, `read_sources`, `update_source`, `read_passages`
- **Processed, the rules of the data**:
  - `objects/ai/agent/agent_rules.rs`: `checked`
  - `objects/ai/rag/rag_rules.rs`: `add_base`, `add_source`, `change_base`, `rename_source`, `replace_source_text`, `search_base`
- **Processed, the AI**:
  - `objects/ai/agent/agent_ai.rs`: `agent_knowledge`
- **Commands** (`panels/views/`):
  - `agents/agent.rs`: `agent_create`, `agent_get`, `agent_save`, `agent_knowledge_test`
  - `agents/knowledge.rs`: `knowledge_base_list`, `knowledge_base_create`, `knowledge_base_save`, `knowledge_source_list`, `knowledge_source_add`, `knowledge_source_replace`, `knowledge_source_rename`, `knowledge_search`
- **Shown** (`ui/src/`):
  - `views/agents/AgentEditor.svelte` ← `agent_get`, `agent_save`, `agent_knowledge_test`
  - `views/agents/AgentList.svelte` ← `agent_create`
  - `views/agents/BaseEditor.svelte` ← `knowledge_base_list`, `knowledge_base_save`, `knowledge_source_list`, `knowledge_source_add`, `knowledge_source_replace`, `knowledge_source_rename`, `knowledge_search`
  - `views/agents/KnowledgeBases.svelte` ← `knowledge_base_list`, `knowledge_base_create`

### `model/evaluation/evaluation_plan.rs`: `EvaluationPlan`, `PlanState`

- **Stored** (`db/models/evaluation/evaluation_plan.rs`): `create_evaluation_plan`, `read_evaluation_plan`, `read_evaluation_plans`, `update_evaluation_plan`, `create_evaluation`
- **Processed, the rules of the data**:
  - `domain/models/evaluation/evaluation_plan/evaluation.rs`: `change_agent`, `evaluation_in_progress`, `new_evaluation`
  - `domain/models/evaluation/evaluation_plan/order.rs`: `check_order`
  - `domain/models/evaluation/evaluation_plan/planning.rs`: `add_document_place`, `check_planning`, `create_plan`, `plan_in_planning`, `plan_to_fill`, `update_plan`
  - `domain/models/evaluation/evaluation_plan/work_units.rs`: `available_controls`, `controls_of`
- **Processed, the AI**:
  - `ai/models/evaluation/work_unit/assessment_answer.rs`: `apply_assessment`
  - `ai/models/evaluation/work_unit/assessment_question.rs`: `assessing_agent`, `assessment_question`, `question_of`
  - `ai/models/evaluation/work_unit/proposal.rs`: `names_of`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/evaluation/inputs.rs`: `inputs`
- **Commands** (`panels/views/`):
  - `evaluation/evaluations.rs`: `evaluation_list`, `evaluation_create`, `evaluation_get`, `evaluation_set_agent`
  - `evaluation/inputs.rs`: `evaluation_document_give`, `evaluation_proof_give`
  - `evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`
  - `evaluation/work_units.rs`: `evaluation_unit_list`, `evaluation_unit_get`, `evaluation_unit_save`
  - `evaluation_plan/fill.rs`: `plan_fill_preview`, `plan_fill_parameters`, `plan_fill_dependencies`
  - `evaluation_plan/plans.rs`: `plan_list`, `plan_create`, `plan_get`, `plan_save`, `plan_add_document`
  - `evaluation_plan/work_units.rs`: `work_unit_controls`, `work_unit_add`, `work_unit_add_all`, `work_unit_remove`, `work_unit_save`, `work_unit_add_proof`
- **Shown** (`ui/src/`):
  - `views/evaluation/EvaluationInputs.svelte` ← `evaluation_get`, `evaluation_set_agent`, `evaluation_document_give`, `evaluation_proof_give`
  - `views/evaluation/EvaluationRun.svelte` ← `evaluation_assess_preview`, `evaluation_assess`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`, `evaluation_unit_save`
  - `views/evaluation/EvaluationUnits.svelte` ← `evaluation_unit_list`
  - `views/evaluation/Evaluations.svelte` ← `plan_list`, `evaluation_list`, `evaluation_create`
  - `views/evaluation_plan/PlanEditor.svelte` ← `plan_get`
  - `views/evaluation_plan/PlanFill.svelte` ← `plan_fill_preview`
  - `views/evaluation_plan/PlanForm.svelte` ← `plan_get`, `plan_save`, `plan_add_document`
  - `views/evaluation_plan/PlanWorkUnits.svelte` ← `plan_get`, `work_unit_controls`, `work_unit_add`, `work_unit_add_all`, `work_unit_remove`
  - `views/evaluation_plan/PlansTab.svelte` ← `plan_list`, `plan_create`
  - `views/evaluation_plan/WorkUnitEditor.svelte` ← `plan_get`, `work_unit_save`, `work_unit_add_proof`

### `model/evaluation/lab_proof.rs`: `LabProof`

- **Stored** (`db/models/evaluation/lab_proof.rs`): `create_lab_proof`, `read_lab_proof`, `update_lab_proof`
- **Processed, the rules of the data**:
  - `domain/models/evaluation/lab_proof.rs`: `give_proof`
  - `domain/models/evaluation/work_unit/parameters.rs`: `add_proof_place`
- **Processed, the AI**:
  - `ai/models/evaluation/evaluation_plan.rs`: `missing_inputs`
  - `ai/models/evaluation/work_unit/assessment_question.rs`: `proofs_text`, `unit_json`
  - `ai/models/evaluation/work_unit/proposal.rs`: `chosen_proofs`, `current_parameters`, `proof_named`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/evaluation/inputs.rs`: `inputs`
- **Commands** (`panels/views/`):
  - `evaluation/evaluations.rs`: `evaluation_create`, `evaluation_get`
  - `evaluation/inputs.rs`: `evaluation_proof_give`
  - `evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`
  - `evaluation/work_units.rs`: `evaluation_unit_get`
  - `evaluation_plan/work_units.rs`: `work_unit_get`, `work_unit_add_proof`
- **Shown** (`ui/src/`):
  - `views/evaluation/EvaluationInputs.svelte` ← `evaluation_get`, `evaluation_proof_give`
  - `views/evaluation/EvaluationRun.svelte` ← `evaluation_assess_preview`, `evaluation_assess`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`
  - `views/evaluation/Evaluations.svelte` ← `evaluation_create`
  - `views/evaluation_plan/WorkUnitEditor.svelte` ← `work_unit_get`, `work_unit_add_proof`

### `model/evaluation/oscal_standard_document.rs`: `OscalStandardDocument`

- **Stored** (`db/models/evaluation/oscal_standard_document.rs`): `create_oscal_standard_document`, `read_oscal_standard_document`, `read_oscal_standard_documents`, `update_oscal_standard_document`
- **Processed, the rules of the data**:
  - `domain/models/evaluation/evaluation_plan/work_units.rs`: `controls_of`
  - `domain/models/evaluation/oscal_standard_document.rs`: `import_standard`
- **Processed, the AI**:
  - `ai/models/evaluation/work_unit/assessment_question.rs`: `unit_json`
  - `ai/models/evaluation/work_unit/proposal.rs`: `parameters_question`
- **Commands** (`panels/views/`):
  - `evaluation/work_units.rs`: `evaluation_unit_get`
  - `evaluation_plan/plans.rs`: `plan_get`
  - `evaluation_plan/standards.rs`: `standard_list`, `standard_import`
  - `evaluation_plan/work_units.rs`: `work_unit_list`, `work_unit_controls`, `work_unit_add`, `work_unit_add_all`
- **Shown** (`ui/src/`):
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`
  - `views/evaluation_plan/PlanEditor.svelte` ← `plan_get`
  - `views/evaluation_plan/PlanForm.svelte` ← `plan_get`
  - `views/evaluation_plan/PlanWorkUnits.svelte` ← `plan_get`, `work_unit_list`, `work_unit_controls`, `work_unit_add`, `work_unit_add_all`
  - `views/evaluation_plan/StandardsTab.svelte` ← `standard_list`, `standard_import`
  - `views/evaluation_plan/WorkUnitEditor.svelte` ← `plan_get`, `work_unit_list`

### `model/evaluation/product_document.rs`: `ProductDocument`

- **Stored** (`db/models/evaluation/product_document.rs`): `create_product_document`, `read_product_document`, `update_product_document`, `read_document_text`, `read_document_passages`
- **Processed, the rules of the data**:
  - `domain/models/evaluation/evaluation_plan/planning.rs`: `add_document_place`
  - `domain/models/evaluation/product_document.rs`: `give_document`
  - `domain/models/evaluation/work_unit/results.rs`: `check_quotes`, `quotes_found`
- **Processed, the AI**:
  - `ai/models/evaluation/evaluation_plan.rs`: `missing_inputs`
  - `ai/models/evaluation/work_unit/assessment_question.rs`: `documents_text`, `unit_json`
  - `ai/models/evaluation/work_unit/proposal.rs`: `names_of`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/evaluation/inputs.rs`: `inputs`
- **Commands** (`panels/views/`):
  - `evaluation/evaluations.rs`: `evaluation_create`, `evaluation_get`
  - `evaluation/inputs.rs`: `evaluation_document_give`
  - `evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`
  - `evaluation/work_units.rs`: `evaluation_unit_get`, `evaluation_unit_save`
  - `evaluation_plan/plans.rs`: `plan_get`, `plan_add_document`
- **Shown** (`ui/src/`):
  - `views/evaluation/EvaluationInputs.svelte` ← `evaluation_get`, `evaluation_document_give`
  - `views/evaluation/EvaluationRun.svelte` ← `evaluation_assess_preview`, `evaluation_assess`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`, `evaluation_unit_save`
  - `views/evaluation/Evaluations.svelte` ← `evaluation_create`
  - `views/evaluation_plan/PlanEditor.svelte` ← `plan_get`
  - `views/evaluation_plan/PlanForm.svelte` ← `plan_get`, `plan_add_document`
  - `views/evaluation_plan/PlanWorkUnits.svelte` ← `plan_get`
  - `views/evaluation_plan/WorkUnitEditor.svelte` ← `plan_get`

### `model/evaluation/work_unit.rs`: `WorkUnit`, `Verdict`, `Author`, `Quote`

- **Stored** (`db/models/evaluation/work_unit.rs`): `create_work_unit`, `read_work_unit`, `read_work_units`, `update_work_unit`, `delete_work_unit`
- **Processed, the rules of the data**:
  - `domain/models/evaluation/evaluation_plan/evaluation.rs`: `needed_proofs`
  - `domain/models/evaluation/evaluation_plan/order.rs`: `check_order`, `links`, `taken_in_order`, `unit_at`, `work_order`
  - `domain/models/evaluation/evaluation_plan/planning.rs`: `create_plan`, `update_plan`
  - `domain/models/evaluation/evaluation_plan/work_units.rs`: `add_all_work_units`, `add_work_unit`, `available_controls`, `holds`, `remove_work_unit`, `unit_of`
  - `domain/models/evaluation/lab_proof.rs`: `give_proof`
  - `domain/models/evaluation/work_unit/parameters.rs`: `add_proof_place`, `update_work_unit_parameters`
  - `domain/models/evaluation/work_unit/results.rs`: `check_quote`, `check_quotes`, `quotes_found`, `save_results`
- **Processed, the AI**:
  - `ai/models/evaluation/evaluation_plan.rs`: `missing_inputs`, `units_in_order`, `units_to_assess`
  - `ai/models/evaluation/work_unit/assessment.rs`: `assess_work_unit`, `prepared_question`, `written`
  - `ai/models/evaluation/work_unit/assessment_answer.rs`: `apply_assessment`, `badness`, `verdict_of`, `with_dependencies`
  - `ai/models/evaluation/work_unit/assessment_question.rs`: `assessing_agent`, `assessment_question`, `dependencies_text`, `documents_text`, `label_of`, `proofs_text`, `question_of`, `unit_json`, `verdict_name`
  - `ai/models/evaluation/work_unit/filling.rs`: `prepared_question`
  - `ai/models/evaluation/work_unit/proposal.rs`: `apply_dependencies`, `apply_parameters`, `chosen_proofs`, `current_parameters`, `dependencies_question`, `labels`, `parameters_question`, `proof_named`, `question_of`, `summary`
  - `ai/models/session/progress.rs`: `record_step`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/evaluation/inputs.rs`: `inputs`, `needing`
- **Commands** (`panels/views/`):
  - `evaluation/evaluations.rs`: `evaluation_create`, `evaluation_get`
  - `evaluation/inputs.rs`: `evaluation_proof_give`
  - `evaluation/run.rs`: `evaluation_assess_preview`, `evaluation_assess`
  - `evaluation/work_units.rs`: `evaluation_unit_list`, `evaluation_unit_get`, `evaluation_unit_save`
  - `evaluation_plan/fill.rs`: `plan_fill_parameters`, `plan_fill_dependencies`
  - `evaluation_plan/plans.rs`: `plan_create`, `plan_get`, `plan_save`
  - `evaluation_plan/work_units.rs`: `work_unit_list`, `work_unit_controls`, `work_unit_add`, `work_unit_add_all`, `work_unit_remove`, `work_unit_get`, `work_unit_save`, `work_unit_add_proof`
- **Shown** (`ui/src/`):
  - `views/evaluation/EvaluationInputs.svelte` ← `evaluation_get`, `evaluation_proof_give`
  - `views/evaluation/EvaluationRun.svelte` ← `evaluation_assess_preview`, `evaluation_assess`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_get`, `evaluation_unit_save`
  - `views/evaluation/EvaluationUnits.svelte` ← `evaluation_unit_list`
  - `views/evaluation/Evaluations.svelte` ← `evaluation_create`
  - `views/evaluation_plan/PlanEditor.svelte` ← `plan_get`
  - `views/evaluation_plan/PlanForm.svelte` ← `plan_get`, `plan_save`
  - `views/evaluation_plan/PlanWorkUnits.svelte` ← `plan_get`, `work_unit_list`, `work_unit_controls`, `work_unit_add`, `work_unit_add_all`, `work_unit_remove`
  - `views/evaluation_plan/PlansTab.svelte` ← `plan_create`
  - `views/evaluation_plan/WorkUnitEditor.svelte` ← `plan_get`, `work_unit_list`, `work_unit_get`, `work_unit_save`, `work_unit_add_proof`

### `model/session/progress.rs`: `Progress`, `Problem`

- **Stored**: never (a helper model, `docs/data_model.md`)
- **Processed, the AI**:
  - `ai/lock.rs`: `lock_progress`
  - `ai/models/evaluation/evaluation_plan.rs`: `assess_evaluation`, `fill_plan`
  - `ai/models/session/progress.rs`: `record_step`
- **Processed, the session**:
  - `session/state.rs`: `begin_work`, `locks`, `new`, `progress`
- **Commands** (`panels/views/`):
  - `chat/chats.rs`: `chat_ask`, `chat_stop`
  - `evaluation/run.rs`: `evaluation_assess`, `evaluation_assess_progress`, `evaluation_assess_stop`
  - `evaluation/work_units.rs`: `evaluation_unit_save`
  - `evaluation_plan/fill.rs`: `plan_fill_parameters`, `plan_fill_dependencies`, `plan_fill_progress`, `plan_fill_stop`
- **Shown** (`ui/src/`):
  - `views/chat/Conversation.svelte` ← `chat_ask`, `chat_stop`
  - `views/evaluation/EvaluationRun.svelte` ← `evaluation_assess`, `evaluation_assess_progress`, `evaluation_assess_stop`
  - `views/evaluation/EvaluationUnit.svelte` ← `evaluation_unit_save`
  - `views/evaluation_plan/PlanFill.svelte` ← `plan_fill_progress`, `plan_fill_stop`

### `objects/user/program_data/program_data_model.rs`: `Installation`, `Settings`

- **Stored** (`program_data_store.rs`): `create_installation`, `read_installation`, `create_settings`, `read_settings`, `update_settings`
- **Processed, the rules of the data**:
  - `objects/user/user_data/user_data_rules.rs`: `install`, `keys_and_database`
- **Processed, the panels (`panels/logic/`)**:
  - `panels/logic/settings/integrations.rs`: `activate_integration`, `add_integration`
- **Commands** (`panels/views/`):
  - `program_start/installation.rs`: `installation_create`
  - `settings/integrations.rs`: `integration_list`, `integration_create`, `integration_activate`
- **Shown** (`ui/src/`):
  - `views/agents/AgentList.svelte` ← `integration_list`
  - `views/program_start/InstallationForm.svelte` ← `installation_create`
  - `views/settings/IntegrationForm.svelte` ← `integration_create`
  - `views/settings/IntegrationList.svelte` ← `integration_list`, `integration_activate`

### `objects/user/user_data/user_data_model.rs`: `User`

- **Stored** (`user_data_store.rs`): `create_user`, `read_user`, `read_users`, `update_user`
- **Processed, the rules of the data**:
  - `objects/user/user_data/user_data_rules.rs`: `install`, `installed_user`, `keys_and_database`, `log_in`
- **Processed, the session**:
  - `session/state.rs`: `log_in`
- **Commands** (`panels/views/`):
  - `program_start/installation.rs`: `installation_create`
  - `program_start/login.rs`: `login_get`, `login_log_in`
- **Shown** (`ui/src/`):
  - `App.svelte` ← `login_get`
  - `views/program_start/InstallationForm.svelte` ← `installation_create`
  - `views/program_start/LoginForm.svelte` ← `login_log_in`

### `objects/user/user_keys/user_keys_model.rs`: `UserKeys`, `WrappedKey`, `RecoveryWrappedKey`

- **Stored** (`user_keys_store.rs`): `create_user_keys`, `read_user_keys`, `update_user_keys`
- **Processed, the rules of the data**:
  - `objects/user/user_data/user_data_rules.rs`: `keys_and_database`, `log_in`
  - `objects/user/user_keys/user_keys_rules/password.rs`: `unwrap`, `wrap`
  - `objects/user/user_keys/user_keys_rules/recovery.rs`: `unwrap`, `wrap`
- **Commands** (`panels/views/`):
  - `program_start/installation.rs`: `installation_create`
  - `program_start/login.rs`: `login_log_in`
- **Shown** (`ui/src/`):
  - `views/program_start/InstallationForm.svelte` ← `installation_create`
  - `views/program_start/LoginForm.svelte` ← `login_log_in`

