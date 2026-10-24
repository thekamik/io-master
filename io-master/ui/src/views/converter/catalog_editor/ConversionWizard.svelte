<!--
  The element "wizard" of the editor of a catalog (its commands
  conversion_wizard_* in src/views/converter/catalog_editor/catalog_editor.rs):
  the conversion of the documents of a standard into the catalog, step by
  step (step 50.2 of docs/history.md):

  1. Table of contents: the AI finds its pages, the evaluator confirms them;
  2. Groups: the AI proposes them from the table of contents;
  3. Controls: group by group, the pages found by the program, the AI asked;
  4. Review: what is left to check; "Finish".

  The proposals are reviewed beside it (Proposals, placed by the editor).
  The stage is kept by the program, so a conversion can be left and resumed.
  After a change it tells the editor (onchanged), whose elements read again;
  the editor counts up `version`, and the wizard reads again.
-->
<script lang="ts">
  import { onMount } from "svelte";

  import { sentence } from "../../../lib/api/errors";
  import * as catalogEditor from "../../../lib/api/converter/catalog_editor";
  import type { PagesView, Stage, WizardView } from "../../../lib/api/converter/catalog_editor";
  import { applyBuildEvent } from "./build_progress";

  let {
    uuid,
    version,
    onchanged,
    onview,
  }: {
    uuid: string;
    version: number;
    onchanged: () => void;
    onview: (documentId: number, page: number) => void;
  } = $props();

  const STAGES: { id: Stage; title: string }[] = [
    { id: "contents", title: "Table of contents" },
    { id: "groups", title: "Groups" },
    { id: "controls", title: "Controls" },
    { id: "review", title: "Review" },
    { id: "finished", title: "Finished" },
  ];

  let wizard: WizardView | null = $state(null);
  let sources: catalogEditor.SourceView[] = $state([]);
  let contents: PagesView = $state({ document_id: 0, first_page: 1, last_page: 1 });
  /** The pages of every group as the evaluator edits them, by the id of the group. */
  let edits: Record<string, PagesView> = $state({});
  let busy = $state(false);
  let problem = $state("");
  let work = $state({ progress: "", log: [] as { done: boolean; text: string }[] });

  const position = $derived(STAGES.findIndex((stage) => stage.id === wizard?.stage));

  onMount(async () => {
    try {
      sources = await catalogEditor.aiBuilderSources();
    } catch (error) {
      problem = sentence(error);
    }
  });

  // The wizard is read when the editor opens and after every change.
  $effect(() => {
    void version;
    load();
  });

  async function load() {
    try {
      show(await catalogEditor.conversionWizardGet(uuid));
    } catch (error) {
      problem = sentence(error);
    }
  }

  function show(view: WizardView) {
    wizard = view;
    contents = view.contents ?? { document_id: sources[0]?.id ?? 0, first_page: 1, last_page: 1 };

    const pages: Record<string, PagesView> = {};
    for (const group of view.groups) {
      pages[group.id] = group.pages ?? { document_id: contents.document_id, first_page: 1, last_page: 1 };
    }
    edits = pages;
  }

  /** Runs a command that gives the wizard back. */
  async function step(command: () => Promise<WizardView>) {
    problem = "";
    work = { progress: "", log: [] };

    try {
      show(await command());
      onchanged();
    } catch (error) {
      problem = sentence(error);
    }
  }

  /** Runs a work of the AI, then the command `after`, if any. */
  async function ask(command: () => Promise<unknown>, after?: () => Promise<WizardView>) {
    busy = true;
    problem = "";
    work = { progress: "", log: [] };

    try {
      await command();
      if (after) {
        show(await after());
      }
    } catch (error) {
      problem = sentence(error);
    } finally {
      busy = false;
      onchanged();
    }
  }

  async function findContents() {
    await ask(
      () =>
        catalogEditor.conversionWizardFindContents(uuid, contents.document_id, (event) =>
          applyBuildEvent(work, event),
        ),
      () => catalogEditor.conversionWizardGet(uuid),
    );

    const found = wizard?.contents;
    work.log.push(
      found
        ? { done: true, text: `The AI found the table of contents on pages ${found.first_page}–${found.last_page}: check them.` }
        : { done: false, text: "The AI found no table of contents: give its pages yourself." },
    );
  }

  function proposeGroups() {
    const range = { ...contents };
    ask(() => catalogEditor.aiBuilderGroups(uuid, range, (event) => applyBuildEvent(work, event)));
  }

  function proposeControls(group: string) {
    const pages = edits[group];
    if (!pages) {
      return;
    }

    ask(
      () => catalogEditor.aiBuilderControls(uuid, group, [{ ...pages }], 0, (event) => applyBuildEvent(work, event)),
      () => catalogEditor.conversionWizardGroupState(uuid, group, "proposed"),
    );
  }

  function title(documentId: number): string {
    return sources.find((source) => source.id === documentId)?.title ?? "a document";
  }
</script>

{#if problem}<p class="problem">{problem}</p>{/if}

{#if wizard}
  <ol class="wizard-stages">
    {#each STAGES as stage, index (stage.id)}
      <li class:active={index === position} class:done={index < position}>{index + 1}. {stage.title}</li>
    {/each}
  </ol>

  {#if wizard.stage === "contents"}
    <h3>The table of contents</h3>
    <p class="note">
      The AI reads the first pages of the document and finds its table of contents. Check the pages in the viewer and
      correct them if needed: the groups are read from them.
    </p>
    <label>
      Document
      <select bind:value={contents.document_id}>
        {#each sources as source (source.id)}<option value={source.id}>{source.title}</option>{/each}
      </select>
    </label>
    <div class="buttons">
      <button type="button" onclick={findContents} disabled={busy || sources.length === 0}>Find it with the AI</button>
    </div>
    <div class="range">
      <label>Pages <input type="number" min="1" bind:value={contents.first_page} /></label>
      <label>to <input type="number" min="1" bind:value={contents.last_page} /></label>
      <button type="button" class="small" onclick={() => onview(contents.document_id, contents.first_page)}>
        Show
      </button>
    </div>
    <div class="buttons">
      <button type="button" onclick={() => step(() => catalogEditor.conversionWizardContents(uuid, { ...contents }))}>
        Keep these pages
      </button>
      <button
        type="button"
        class="primary"
        disabled={!wizard.contents}
        onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "groups"))}
      >
        Next: the groups
      </button>
    </div>
  {:else if wizard.stage === "groups"}
    <h3>The groups</h3>
    <p class="note">
      The AI reads the table of contents ({title(contents.document_id)}, pages {contents.first_page}–{contents.last_page})
      and proposes the groups of the catalog. Accept, correct or reject every proposal beside; then go on to the
      controls.
    </p>
    <div class="buttons">
      <button type="button" onclick={proposeGroups} disabled={busy}>Propose the groups</button>
      <button type="button" onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "contents"))}>
        Back
      </button>
      <button
        type="button"
        class="primary"
        onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "controls"))}
      >
        Next: the controls
      </button>
    </div>
  {:else if wizard.stage === "controls"}
    <h3>The controls, group by group</h3>
    <p class="note">
      The program found where every group starts. Correct the pages if needed, let the AI read them, review the
      proposals beside, and mark the group done.
    </p>
    {#if wizard.groups.length === 0}
      <p class="note">The catalog has no group yet: go back and accept the groups.</p>
    {:else}
      <ul class="wizard-groups">
        {#each wizard.groups as group (group.id)}
          {@const pages = edits[group.id]}
          <li class:finished={group.state === "done"}>
            <div class="wizard-group-head">
              <strong>{group.title}</strong>
              <span class="badge">{group.state}</span>
              <span class="note">{group.controls} {group.controls === 1 ? "control" : "controls"}</span>
            </div>
            {#if pages}
              <div class="range">
                <label>Pages <input type="number" min="1" bind:value={pages.first_page} /></label>
                <label>to <input type="number" min="1" bind:value={pages.last_page} /></label>
                <button
                  type="button"
                  class="small"
                  onclick={() => step(() => catalogEditor.conversionWizardGroupPages(uuid, group.id, { ...pages }))}
                >
                  Keep
                </button>
                <button type="button" class="small" onclick={() => onview(pages.document_id, pages.first_page)}>
                  Show
                </button>
                {#if !group.pages}<span class="note">its start was not found: give its pages and keep them</span>{/if}
              </div>
            {/if}
            <div class="buttons">
              <button type="button" class="small" disabled={busy || !group.pages} onclick={() => proposeControls(group.id)}>
                Propose the controls
              </button>
              <button
                type="button"
                class="small"
                disabled={group.state === "done"}
                onclick={() => step(() => catalogEditor.conversionWizardGroupState(uuid, group.id, "done"))}
              >
                This group is done
              </button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
    <div class="buttons">
      <button type="button" onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "groups"))}>
        Back
      </button>
      <button type="button" class="primary" onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "review"))}>
        Next: the review
      </button>
    </div>
  {:else if wizard.stage === "review"}
    <h3>The review</h3>
    <ul class="plain">
      <li>Proposals of the AI not decided yet: <strong>{wizard.review.proposals}</strong></li>
      <li>Quotes not found on their page: <strong>{wizard.review.quotes_not_found}</strong></li>
      <li>Groups without a control: <strong>{wizard.review.groups_without_controls}</strong></li>
    </ul>
    <p class="note">
      Check them in the manual editing if needed. When the catalog is finished, the view Evaluation plan can use it.
    </p>
    <div class="buttons">
      <button type="button" onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "controls"))}>
        Back
      </button>
      <button type="button" class="primary" onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "finished"))}>
        Finish
      </button>
    </div>
  {:else}
    <h3>Finished</h3>
    <p class="note">The catalog is finished: the view Evaluation plan uses it. It can still be changed by hand.</p>
    <div class="buttons">
      <button type="button" onclick={() => step(() => catalogEditor.conversionWizardStage(uuid, "review"))}>
        Back to the review
      </button>
    </div>
  {/if}

  {#if busy || work.progress || work.log.length > 0}
    <div class="load-log">
      {#if work.progress}<p class="note">{work.progress}</p>{/if}
      {#each work.log as line, index (index)}<p class={line.done ? "done" : "problem"}>{line.text}</p>{/each}
      {#if busy}
        <button type="button" class="small danger" onclick={catalogEditor.aiBuilderStop}>Stop</button>
      {/if}
    </div>
  {/if}
{/if}
