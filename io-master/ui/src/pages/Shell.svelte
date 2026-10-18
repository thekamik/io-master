<!--
  The frame of every page of a logged-in evaluator: the warning
  bars at the top, the sidebar on the left (the mark, the pages, the
  user), the page with its title on the right.

  The icons are drawn here as simple SVG lines (no images, no icon
  library), in the colour of the text around them.
-->
<script lang="ts">
  import { logOut, type AppView } from "../lib/api/app";
  import Chat from "./Chat.svelte";
  import Documentation from "./Documentation.svelte";
  import Evaluation from "./Evaluation.svelte";
  import Scan from "./Scan.svelte";
  import Plans from "./Plans.svelte";
  import Settings from "./Settings.svelte";

  let { app, onchange }: { app: AppView; onchange: (app: AppView) => void } = $props();

  /** The page chosen in the sidebar; null: none chosen yet. */
  let chosen: string | null = $state(null);

  /** The page shown: the chosen one, or the first of the role. */
  let active = $derived(chosen ?? app.pages[0]?.id ?? "");

  let title = $derived(app.pages.find((page) => page.id === active)?.title ?? "");

  /** The line under the title of each page. */
  const subtitles: Record<string, string> = {
    chat: "Ask the model of your active AI connection.",
    evaluation: "Carry out an evaluation scheme for a product.",
    plans: "The standards in NIST OSCAL and the plan of an evaluation, with the documents of the standards they come from.",
    documentation: "Load and analyse the documentation of the manufacturer for the evaluations.",
    scan: "The serial ports of this computer, and the access to them.",
    settings: "Your account, the connections to AI services, and this installation.",
    admin: "The password resets of the workstations.",
  };

  async function leave() {
    onchange(await logOut());
  }
</script>

{#snippet icon(name: string)}
  <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
    {#if name === "chat"}
      <path d="M4 5.5h16v10.5H10l-5 4v-4H4z" />
      <path d="M8 9.5h8M8 12.5h5" />
    {:else if name === "evaluation"}
      <path d="M9 4h6v3H9z" />
      <path d="M7 5.5H5.5v14h13v-14H17" />
      <path d="M8.5 13l2.2 2.2 4.8-4.7" />
    {:else if name === "plans"}
      <rect x="3.5" y="4" width="6" height="5" rx="1.2" />
      <rect x="14.5" y="4" width="6" height="5" rx="1.2" />
      <rect x="9" y="15" width="6" height="5" rx="1.2" />
      <path d="M6.5 9v2.5h11V9M12 11.5V15" />
    {:else if name === "documentation"}
      <path d="M6 3.5h8l4 4v13H6z" />
      <path d="M14 3.5v4h4M9 12h6M9 15h6M9 18h4" />
    {:else if name === "scan"}
      <rect x="4" y="8" width="16" height="9" rx="2" />
      <path d="M8 12h.01M12 12h.01M16 12h.01M9 4v4M15 4v4" />
    {:else if name === "settings"}
      <path d="M4 7h9M17 7h3M4 17h3M11 17h9" />
      <circle cx="15" cy="7" r="2" />
      <circle cx="9" cy="17" r="2" />
    {:else if name === "admin"}
      <path d="M12 3.5l7 3v5c0 4.5-3 7.5-7 9-4-1.5-7-4.5-7-9v-5z" />
      <path d="M9.5 12l1.8 1.8 3.4-3.6" />
    {:else if name === "logout"}
      <path d="M14 4h5v16h-5M10 8l-4 4 4 4M6 12h10" />
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

    <div class="user">
      <span class="avatar">{app.user?.charAt(0) ?? "?"}</span>
      <span class="who">{app.user}</span>
      <button class="ghost" onclick={leave} title="Log out">{@render icon("logout")}</button>
    </div>
  </nav>

  <main>
    <header class="page-head">
      <h1>{title}</h1>
      {#if subtitles[active]}<p>{subtitles[active]}</p>{/if}
    </header>

    {#if active === "chat"}
      <Chat />
    {:else if active === "evaluation"}
      <Evaluation />
    {:else if active === "plans"}
      <Plans />
    {:else if active === "documentation"}
      <Documentation />
    {:else if active === "scan"}
      <Scan />
    {:else if active === "settings"}
      <Settings {app} />
    {:else}
      <!-- The panel of the master: not built yet (step 11 of docs/todo.md). -->
      <p class="note">Password reset requests are approved here (step 11).</p>
    {/if}
  </main>
</div>
