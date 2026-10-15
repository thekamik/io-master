// IO Master: the script of the chat page (web_ui/chat_page.rs).
//
// It sends a message with fetch() and shows the reply while it is
// written. The server answers with server-sent events:
//
//   event: chat   {"id": 7, "url": "/chat/7", "title": "..."}
//   event: text   {"text": "a piece of the reply"}
//   event: end    {"ending": "complete", "notice": null}
//
// "Stop" aborts the request; the server then stops the reply and
// stores what was written up to then.
//
// Text from the server is only ever set as textContent, never as
// HTML, so a reply can never become markup or a script.

"use strict";

(() => {
    const form = document.getElementById("ask");

    if (!form) {
        return;
    }

    const messages = document.getElementById("messages");
    const sendButton = document.getElementById("send");
    const stopButton = document.getElementById("stop");
    const status = document.getElementById("status");
    const chatField = form.elements.chat;
    const textField = form.elements.text;

    // The request being answered, so that "Stop" can abort it.
    let running = null;

    // The local time as the server shows it: "2026-10-14 15:04".
    function now() {
        const date = new Date();
        const two = (number) => String(number).padStart(2, "0");

        return `${date.getFullYear()}-${two(date.getMonth() + 1)}-${two(date.getDate())} ` +
            `${two(date.getHours())}:${two(date.getMinutes())}`;
    }

    // Adds a message to the conversation and returns its parts.
    function addMessage(kind, who, text) {
        const article = document.createElement("article");
        const header = document.createElement("header");
        const body = document.createElement("div");

        article.className = `message ${kind}`;
        header.textContent = `${who} · ${now()}`;
        body.className = "text";
        body.textContent = text;

        article.append(header, body);
        messages.append(article);
        article.scrollIntoView({ block: "end" });

        return { article, header, body };
    }

    // Adds the line under a reply that did not end normally.
    function addNotice(article, text) {
        const notice = document.createElement("p");

        notice.className = "ending";
        notice.textContent = text;
        article.append(notice);
    }

    // Shows the chat the server put the message in: in the hidden
    // field, the address bar and the list on the left.
    function showChat(chat) {
        if (chatField.value === String(chat.id)) {
            return;
        }

        chatField.value = String(chat.id);
        history.replaceState(null, "", chat.url);
        document.getElementById("chat-title").textContent = chat.title;

        const item = document.createElement("li");
        const link = document.createElement("a");

        link.href = chat.url;
        link.className = "active";
        link.textContent = chat.title;
        item.append(link);
        document.getElementById("chats").prepend(item);
    }

    // Reads one event block ("event: x\ndata: {...}").
    function parseEvent(block) {
        let name = "";
        const data = [];

        for (const line of block.split("\n")) {
            if (line.startsWith("event:")) {
                name = line.slice(6).trim();
            } else if (line.startsWith("data:")) {
                data.push(line.slice(5).replace(/^ /, ""));
            }
        }

        return { name, data: data.length ? JSON.parse(data.join("\n")) : null };
    }

    // Switches between "writing" and "ready".
    function setRunning(controller) {
        running = controller;
        sendButton.disabled = controller !== null;
        stopButton.hidden = controller === null;
        status.textContent = controller ? "The model is writing…" : "";
    }

    async function send(event) {
        event.preventDefault();

        const text = textField.value.trim();

        if (!text || running) {
            return;
        }

        const body = new URLSearchParams(new FormData(form));
        const controller = new AbortController();

        addMessage("evaluator", "You", text);
        const reply = addMessage("model", "Model", "");
        setRunning(controller);

        let ended = false;

        try {
            const response = await fetch(form.action, {
                method: "POST",
                body,
                signal: controller.signal,
                headers: { Accept: "text/event-stream" },
            });

            if (!response.ok) {
                addNotice(reply.article, await response.text());
                return;
            }

            textField.value = "";

            const reader = response.body.getReader();
            const decoder = new TextDecoder();
            let buffer = "";

            for (;;) {
                const { value, done } = await reader.read();

                if (done) {
                    break;
                }

                buffer += decoder.decode(value, { stream: true }).replace(/\r\n/g, "\n");

                let end;

                while ((end = buffer.indexOf("\n\n")) >= 0) {
                    const { name, data } = parseEvent(buffer.slice(0, end));

                    buffer = buffer.slice(end + 2);

                    if (name === "chat") {
                        showChat(data);
                    } else if (name === "text") {
                        reply.body.textContent += data.text;
                        reply.article.scrollIntoView({ block: "end" });
                    } else if (name === "end") {
                        ended = true;

                        if (data.notice) {
                            addNotice(reply.article, data.notice);
                        }
                    }
                }
            }

            if (!ended) {
                addNotice(reply.article, "The connection to IO Master broke off; reload the page to see what was stored.");
            }
        } catch (error) {
            if (error.name === "AbortError") {
                addNotice(reply.article, "Stopped.");
            } else {
                addNotice(reply.article, `IO Master cannot be reached: ${error.message}`);
            }
        } finally {
            setRunning(null);
        }
    }

    form.addEventListener("submit", send);

    stopButton.addEventListener("click", () => {
        if (running) {
            running.abort();
        }
    });

    // Enter sends, Shift+Enter starts a new line.
    textField.addEventListener("keydown", (event) => {
        if (event.key === "Enter" && !event.shiftKey && !event.isComposing) {
            event.preventDefault();
            form.requestSubmit();
        }
    });
})();
