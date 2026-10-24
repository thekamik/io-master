<!--
  The view Evaluation lab (step 52 of docs/todo.md; its commands in
  src/views/evaluation_lab/evaluation_lab.rs): a work package of the
  Evaluation plan imported and carried out for one product. The list of
  evaluations (EvaluationList) or one evaluation (EvaluationScreen), whose
  task opens its screen (LabTask). The AI proposes, the evaluator decides;
  the result is the report, exported out of the program. Presentation only.
-->
<script lang="ts">
  import EvaluationList from "./EvaluationList.svelte";
  import EvaluationScreen from "./EvaluationScreen.svelte";

  /** The evaluation open, if any. */
  let evaluation: string | null = $state(null);
</script>

{#if evaluation}
  <EvaluationScreen {evaluation} onclose={() => (evaluation = null)} />
{:else}
  <EvaluationList onopen={(uuid) => (evaluation = uuid)} />
{/if}
