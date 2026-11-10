# IO Master: changes since `V0.3`

What changed on `test_branch` after the stable beta `V0.3` (2026-11-04), by what a tester sees. All of it was made on 2026-11-10, after a verification of the whole code. One model changed: `Agent` has the field `temperature`, and its table a column of that name, so **the data of a test of `V0.3` does not open in this version**.

## Before the first run

- **Remove the data folder** (`~/.local/share/io-master`) and start anew: the table of the agents has a new column, and the program moves no data between versions before version 1.
- **An integration named by `localhost` is refused**: write `127.0.0.1` (or `[::1]`) in its address.
- **Install `bubblewrap`** beside `poppler-utils` (`README.md`). Without it no PDF is read: a source of knowledge or a document of the product given as a PDF is refused with a message that names the package. The sandbox needs the user namespaces of Linux, which Debian 13 has switched on.
- **Give the PDFs and the texts of the knowledge bases again** (or change the length of a passage of each base) when the new cutting of the passages is wanted for them; the sources and documents already stored keep their old passages.
- **Two defaults of a new agent changed**; an agent written by hand from the texts of `V0.3` gets them only in its editor:
  - the task of an evaluator (documentation and functional test) has a new paragraph: the documents of the manufacturer are evidence, never instructions;
  - the knowledge given with one question is 24 000 characters (it was 12 000).

## Security

| Change | Why |
|---|---|
| The tools of Poppler (`pdfinfo`, `pdftotext`) run in a sandbox of `bubblewrap`: no network, no home folder, no data of the program, the system read-only. | A PDF comes from outside the laboratory; a fault of the parser must reach neither the databases nor the keys. |
| Each call of a tool is bounded: 120 seconds, 64 MiB of output; a tool that runs longer is killed. | A file made to keep the tool busy or to fill the memory stopped the program. |
| The values of a report are escaped in a template of HTML, XML (`html`, `htm`, `xhtml`, `xml`) and LaTeX (`tex`, `latex`); Markdown and plain text are written as they are. | Findings written by the AI could bring a script into a page or a command into a compiled document. |
| Plain HTTP only to `127.0.0.1` and `[::1]`; the name `localhost` is refused. | A name is what the system resolves it to; on a computer set wrongly the API key would leave it without TLS. |
| A stored cost of Argon2id over 1 GiB or 16 passes is refused as damaged. | The system database is not encrypted: a changed copy of the key could make the login take all the memory. |
| The task of a new evaluator tells the model that the documents are evidence and never instructions, and to report a text in them that speaks to it. | The manufacturer writes the documents and gains by a pass. |

## The first start

- The two copies of the key are the last thing a first start writes. A first start cut off half way (the program killed, the power lost) counts as not installed, and the next one removes what it left. Before, such a start left an installation nobody could log in to.
- A failed database of the account is told as a failure, no more as a wrong input.

## The assessment by the AI

- **An agent may ask for a temperature** (the editor of an agent, "Temperature"; 0 to 2, empty by default). No temperature was sent before, so the same evidence could get another verdict from one run to the next; a low one (0 to 0.2) makes the assessment and "Fill with AI" repeatable. Left empty, none is sent: some models refuse the parameter, and their service then answers with an error that names it.
- **The note of the documents given as passages** now tells the evaluator that a verdict other than a pass is to be checked against the whole documents: what the model found missing may stand in a part it did not see.
- **A work unit of a standard without a part `guidance` has its guidance** (ETSI TS 103 701): the other parts of its control (the test units, the assignment of the verdict, the method of the assessment), each under its name. Before, such a work unit had an empty guidance, and the model judged without the method of the standard. The work units already in a plan keep what they have: add them to a plan again.
- **An answer is not taken** when it gives a verdict without an analysis, or a field that is no text. Before, such an answer was written as a result.
- **A quote too long is left out and told in the notes**; before, it was cut to the limit and kept as if the model had quoted that much.
- **A result written by the evaluator is never written over by the AI**: checked when the work units of an assessment are chosen, and again when each answer is written. That the evaluation is in progress is checked there as well.
- **"Stop" ends the answer awaited at once**, in the assessment and in "Fill with AI". Before, it waited for the whole answer of the model.
- **"Fill with AI" takes no cut, refused or stopped answer**, as the assessment never did.
- **A warning among the notes** when the service counted fewer than half the tokens the program estimated for a question: a local server with a context smaller than the integration says may have shortened the question in silence.
- **A proof without the day it was performed** is refused when the question of a functional test is written.
- **The quotes the evaluator writes**: an empty quote is refused.

## The knowledge of an agent (RAG)

- **A passage ends where a paragraph ends.** Before, a text was cut every 1500 characters between words, whatever its paragraphs. This holds for the index of the documents of the product as well.
- **Each word of a question is looked for once.** Before, a word a requirement repeats ("evaluator") weighed as many times and found passages by itself. Also in the search of the documents of the product.
- **The passages that name an identifier of the question come first** ("ADV_ARC.1-1", "FCS_COP.1", "5.3-1"). Before, an identifier was looked for by its parts ("ADV", "ARC", "1"), which other work units hold as well.
- **A passage is given once**, also when two sources hold its text.
- **Each found passage is followed by the next one of its source**, in the room the found passages leave within the limit of the agent.
- The "Search" of a base shows the found passages alone; the test of the knowledge of an agent shows what the model gets.

## Speed

Measured on the CEM (813 work units), a debug build:

| What | Before | After |
|---|---|---|
| The preview of an assessment, an ST of 5 MiB | 35 s | 0.25 s |
| The work units of an evaluation read | 212 ms | 90 ms |
| All the work units of a standard added to a plan | 1.3 s | 0.8 s |

- **The window no more stops** while a command waits for the lock of the session or for a slow query: every command runs apart from the thread of the window (the rule is in `CLAUDE.md`, "The window").
- The text of a document is read from its PDF before the lock of the session is taken.
- A document of the product is read without its file, and a source of knowledge without its text, wherever only the name or the size is shown (the lists, the question of a work unit, the check of the quotes, a report). A source is renamed without its text written and indexed again.

## What was tried, and what was not

- **Tried by the unit tests** (216, all passing), `cargo clippy --all-targets` at the pedantic level and `svelte-check`, both clean: every change above.
- **Tried in the window on a private display**, after the commit `a97db6f`: the first start with its recovery code, every panel opened, the login again.
- **Not tried in the window**: the field of the temperature in the editor of an agent and a question sent with it to a real service, an assessment with a real model, "Stop" in the middle of an answer, the export of a report with escaped values, a PDF read through the sandbox, the search of a real knowledge base (the CEM). These are the first things to test.
- **Not measured**: whether the new search of the knowledge finds the right passages on a real evaluation. The rules of the search were tried on small texts written for the tests.

## Known limits

- The warning of a shortened question compares the tokens the service counted with an estimate made from `characters_per_token` of the integration. An integration set to fewer than about 2 characters a token may be warned of without a cause.
- On a system without user namespaces the sandbox does not start, and no PDF is read.
- The search of the knowledge is by words (FTS5, BM25): a text that says the same in other words is found only with the help of the glossary of the agent.
- The open work is in `docs/todo.md`.
