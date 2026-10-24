<!--
  The fields of an output (name, format, condition, target, instructions for
  its AI), bound to the form of the caller: used by BlockForm (an output of
  a task) and StandardOutputs (an output of the template of a project), so
  that both write an output the same way. Presentation only.
-->
<script lang="ts">
  import type { StandardOutputForm } from "../../lib/api/evaluation_plan";

  let { fields = $bindable() }: { fields: StandardOutputForm } = $props();
</script>

<label>Name <input bind:value={fields.name} required placeholder="pETR, Observation Report, …" /></label>
<label>
  Format
  <select bind:value={fields.format}>
    <option value="text">a text file</option>
    <option value="latex">a LaTeX template</option>
  </select>
</label>
<label>
  Filled
  <select bind:value={fields.condition}>
    <option value="always">always</option>
    <option value="not_met">only when the requirement is not met</option>
    <option value="met">only when the requirement is met</option>
  </select>
</label>
<label>Target <textarea bind:value={fields.target} rows="2" placeholder="The file or the template, and its section"></textarea></label>
<label>
  Instructions for its AI
  <textarea bind:value={fields.instructions} rows="3" placeholder="How to adapt the result to this document"></textarea>
</label>
