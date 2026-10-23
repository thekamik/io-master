<!--
  The view Shell (src/views/shell/): the frame of every page of a
  logged-in evaluator: the warning bars at the top, the sidebar on the
  left (the mark, the pages, the user menu), the page with its title on
  the right. Which page is open is presentation, kept here.

  The icons are drawn here as simple SVG lines (no images, no icon
  library), in the colour of the text around them.
-->
<script lang="ts">
  import type { ProgramView } from "../../lib/api/shell";
  import Chat from "../chat/Chat.svelte";
  import EvaluationLab from "../evaluation_lab/EvaluationLab.svelte";
  import EvaluationPlan from "../evaluation_plan/EvaluationPlan.svelte";
  import Settings from "../settings/Settings.svelte";
  import UserMenu from "../users/UserMenu.svelte";
  import Users from "../users/Users.svelte";

  let { app, onloggedout }: { app: ProgramView; onloggedout: () => void } = $props();

  /** The page chosen in the sidebar; null: none chosen yet. */
  let chosen: string | null = $state(null);

  /** The page shown: the chosen one, or the first of the role. */
  let active = $derived(chosen ?? app.pages[0]?.id ?? "");

  let title = $derived(app.pages.find((page) => page.id === active)?.title ?? "");

  /** The line under the title of each page. */
  const subtitles: Record<string, string> = {
    chat: "Ask the model of your active AI connection.",
    evaluation_plan: "The standards in NIST OSCAL and the plan of an evaluation, with the documents of the standards they come from.",
    evaluation_lab: "Carry out an evaluation plan for a product, with what the manufacturer and the laboratory deliver.",
    users: "Your account: the password that opens your encrypted database.",
    settings: "The connections to AI services, and this installation.",
    admin: "The password resets of the workstations.",
  };
</script>

{#snippet icon(name: string)}
  <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
    {#if name === "chat"}
      <path d="M4 5.5h16v10.5H10l-5 4v-4H4z" />
      <path d="M8 9.5h8M8 12.5h5" />
    {:else if name === "evaluation_lab"}
      <path d="M9 4h6v3H9z" />
      <path d="M7 5.5H5.5v14h13v-14H17" />
      <path d="M8.5 13l2.2 2.2 4.8-4.7" />
    {:else if name === "evaluation_plan"}
      <rect x="3.5" y="4" width="6" height="5" rx="1.2" />
      <rect x="14.5" y="4" width="6" height="5" rx="1.2" />
      <rect x="9" y="15" width="6" height="5" rx="1.2" />
      <path d="M6.5 9v2.5h11V9M12 11.5V15" />
    {:else if name === "users"}
      <circle cx="12" cy="8" r="3.5" />
      <path d="M5 20c0-3.9 3.1-7 7-7s7 3.1 7 7" />
    {:else if name === "settings"}
      <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
      <circle cx="15" cy="7" r="2" />
      <circle cx="9" cy="17" r="2" />
    {:else if name === "admin"}
      <path d="M12 3.5l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9v-5z" />
      <path d="M9.5 12l1.8 1.8 3.4-3.6" />
    {:else if name === "warning"}
      <path d="M12 4l9 16H3z" />
      <path d="M12 10v4M12 17h.01" />
    {/if}
  </svg>
{/snippet}

{#each app.warnings as warning (warning.text)}
  <div class="bar {warning.level}">{@render icon("warning")} {warning.text}</div>
{/each}

<div class="layout">
  <nav class="sidebar">
    <div class="brand">
      <span class="mark">IO</span>
      <div>
        <div class="name">IO Master</div>
        <div class="role">{app.role}</div>
      </div>
    </div>

    {#each app.pages as page (page.id)}
      <button class="nav-item" class:active={page.id === active} onclick={() => (chosen = page.id)}>
        {@render icon(page.id)}
        {page.title}
      </button>
    {/each}

    <UserMenu user={app.user} {onloggedout} />
  </nav>

  <main>
    <header class="page-head">
      <h1>{title}</h1>
      {#if subtitles[active]}<p>{subtitles[active]}</p>{/if}
    </header>

    {#if active === "chat"}
      <Chat />
    {:else if active === "evaluation_plan"}
      <EvaluationPlan />
    {:else if active === "evaluation_lab"}
      <EvaluationLab />
    {:else if active === "users"}
      <Users user={app.user} />
    {:else if active === "settings"}
      <Settings {app} />
    {:else if active === "admin"}
      <!-- The panel of the master: not built yet (step 11 of docs/todo.md). -->
      <p class="note">Password reset requests are approved here (step 11).</p>
    {/if}
  </main>
</div>
