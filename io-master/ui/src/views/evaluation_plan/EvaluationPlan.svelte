<!--
  The view Evaluation plan (src/views/evaluation_plan/): it works only on the
  documents of NIST OSCAL (catalogs, mappings, profiles, assessment plans):
  the list, the import, the export, the deletion. Later the document shown
  graphically and turned into the flow of data of the evaluation, used by the
  view Evaluation lab (step 50 of docs/todo.md). The documents are made from
  the PDFs of a standard in the view Converter.
-->
<script lang="ts">
  import OscalDocuments from "./OscalDocuments.svelte";
</script>

<OscalDocuments />
