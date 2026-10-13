/*
 * The "ai-config" command: a short series of questions that sets
 * up the AI integration of the program.
 *
 *     Step 1   which provider       a numbered list
 *     Step 2   which model          a numbered list, or any name
 *     Step 3   the API key          typed without being shown
 *     Step 4   test it?             one short message, on request
 *
 * Every question shows its default in square brackets; pressing
 * Enter accepts it. The defaults are the current configuration,
 * so running the command again and pressing Enter four times
 * changes nothing.
 *
 * What is stored where
 * --------------------
 * The provider and the model go to the AI configuration file,
 * im_config/ai_config.json. The API key goes to the keyring of
 * the system and nowhere else; the file only holds the name of
 * the keyring entry.
 *
 * Nothing is stored until step 3 is answered. Cancelling before
 * that (Ctrl+C, Ctrl+D) leaves everything as it was.
 *
 * The file is organized in this order:
 *
 *     input      reading an answer, visible or hidden
 *     steps      one ask_*() function per question
 *     test       sending the test message
 *     command    command_ai_config(), which runs the steps
 */

// TODO: The less common settings (reply length limit, system
//       prompt, custom address and certificates) have no question
//       here and are changed in im_config/ai_config.json. Add
//       an "advanced" path when they are needed more often.

#include "cli/ai_config.h"

#include "ai/ai_settings.h"
#include "ai/keyring.h"

#include <errno.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <termios.h>
#include <unistd.h>

/*
 * Message sent by the test, and the limit of its reply. The
 * limit leaves room for the thinking that some models do before
 * they answer, while keeping the cost of the test negligible.
 */
#define TEST_MESSAGE "Reply with the single word: OK"
#define TEST_MAX_TOKENS 512

/*
 * Set by Ctrl+C (SIGINT).
 *
 * The handler is installed without SA_RESTART, so Ctrl+C
 * interrupts a question that is waiting for an answer; the code
 * after the read then cleans up, which matters while the echo of
 * the terminal is turned off for typing the key. During the test
 * the flag cancels the request.
 */
static volatile sig_atomic_t interrupted = 0;

/*
 * SIGINT handler. It only sets the flag.
 */
static void handle_signal(int signal) {
    (void)signal; // Unused parameter
    interrupted = 1;
}

/*
 * Reads one visible answer from the terminal. The line break
 * and surrounding spaces are removed, and only the first word is
 * kept. buffer has size characters.
 *
 * Returns 0 on success (the answer may be empty, which means
 * "use the default") and -1 when the user cancelled: Ctrl+C,
 * the end of input, or a line that does not fit.
 */
static int read_answer(char *buffer, size_t size)
{
    char line[256];

    fflush(stdout);

    if (fgets(line, sizeof(line), stdin) == NULL) {
        /*
         * Ctrl+C or the end of input.
         */
        clearerr(stdin);
        printf("\n");
        return -1;
    }

    /*
     * No line break in the buffer: the line was too long.
     * Drop the rest of it.
     */
    if (strchr(line, '\n') == NULL && !feof(stdin)) {
        int character;

        while ((character = getchar()) != '\n' && character != EOF) {
        }

        return -1;
    }

    /*
     * Cut the word out of the line: skip leading spaces and
     * stop at the first space or line break after it.
     */
    const char *start = line + strspn(line, " \t");
    size_t length = strcspn(start, " \t\r\n");

    if (length >= size) {
        return -1;
    }

    memcpy(buffer, start, length);
    buffer[length] = '\0';

    return 0;
}

/*
 * Reads one answer from the terminal without showing what is
 * typed, as for a password.
 *
 * The line break is removed. buffer has size characters.
 *
 * Returns 0 on success (the answer may be empty) and -1 when
 * the user cancelled: Ctrl+C, the end of input, or a line that
 * does not fit.
 */
static int read_hidden_answer(char *buffer, size_t size)
{
    struct termios original;
    bool echo_disabled = false;

    fflush(stdout);

    /*
     * Turn the echo of the terminal off. When the input is not
     * a terminal (a pipe), there is no echo to turn off.
     */
    if (tcgetattr(STDIN_FILENO, &original) == 0) {
        struct termios hidden = original;

        hidden.c_lflag &= ~(tcflag_t)ECHO;

        if (tcsetattr(STDIN_FILENO, TCSAFLUSH, &hidden) == 0) {
            echo_disabled = true;
        }
    }

    char *line = fgets(buffer, (int)size, stdin);

    /*
     * Always restore the terminal, also after Ctrl+C.
     */
    if (echo_disabled) {
        tcsetattr(STDIN_FILENO, TCSAFLUSH, &original);

        /*
         * The line break typed by the user was not shown.
         */
        printf("\n");
    }

    if (line == NULL) {
        clearerr(stdin);
        return -1;
    }

    size_t length = strcspn(buffer, "\r\n");

    /*
     * No line break in the buffer: the line was too long.
     * Drop the rest of it.
     */
    if (buffer[length] == '\0' && !feof(stdin)) {
        int character;

        while ((character = getchar()) != '\n' && character != EOF) {
        }

        explicit_bzero(buffer, size);
        return -1;
    }

    buffer[length] = '\0';

    return 0;
}

/*
 * Converts an answer to a choice from a numbered list.
 *
 * Returns the number (1 to count) when the answer consists of
 * digits and is in range, and 0 otherwise.
 */
static size_t parse_choice(const char *answer, size_t count)
{
    if (answer[0] == '\0' ||
        answer[strspn(answer, "0123456789")] != '\0') {
        return 0;
    }

    unsigned long number = strtoul(answer, NULL, 10);

    return (number >= 1 && number <= count) ? (size_t)number : 0;
}

/*
 * Returns the name that a provider is shown under: the name
 * users know it by when there is one ("claude"), otherwise its
 * own name ("gemini").
 */
static const char *display_name(const ai_provider_info_t *provider)
{
    return provider->alias != NULL ? provider->alias : provider->name;
}

/*
 * Step 1: which provider.
 *
 * Shows the supported providers as a numbered list and reads
 * a choice: the number, or the name. Enter keeps the provider
 * of the current configuration. An answer that matches nothing
 * is asked again.
 *
 * Returns the chosen provider, or NULL when the user cancelled.
 */
static const ai_provider_info_t *ask_provider(
    const ai_settings_t *settings
)
{
    char answer[AI_PROVIDER_NAME_MAX];
    size_t count = ai_provider_count();

    /*
     * Position of the current provider in the list, counted
     * from 1. It is the default answer.
     */
    size_t current = 1;

    printf("Step 1 of 4: choose the AI provider\n\n");

    for (size_t i = 0; i < count; i++) {
        const ai_provider_info_t *provider = ai_provider_get(i);

        if (strcmp(provider->name, settings->config.provider) == 0) {
            current = i + 1;
        }

        printf(
            "  %zu) %-8s %s\n",
            i + 1,
            display_name(provider),
            provider->description
        );
    }

    printf("\n");

    for (;;) {
        printf("Provider [%zu]: ", current);

        if (read_answer(answer, sizeof(answer)) != 0) {
            return NULL;
        }

        /*
         * Enter: the default.
         */
        if (answer[0] == '\0') {
            return ai_provider_get(current - 1);
        }

        /*
         * A number from the list.
         */
        size_t choice = parse_choice(answer, count);

        if (choice != 0) {
            return ai_provider_get(choice - 1);
        }

        /*
         * A name: "gemini", "claude" or "anthropic".
         */
        const ai_provider_info_t *provider = ai_provider_find(answer);

        if (provider != NULL) {
            return provider;
        }

        printf(
            "'%s' is not on the list. Type a number from 1 to %zu.\n",
            answer,
            count
        );
    }
}

/*
 * Step 2: which model.
 *
 * Shows the models that the provider suggests as a numbered list
 * and reads a choice: the number, or the name of any model of
 * the provider, also one that is not on the list.
 *
 * current_model is the model of the current configuration when
 * the provider did not change, or "" when it did; it decides
 * the default answer. model receives the chosen name; it has
 * size characters.
 *
 * Returns 0 on success and -1 when the user cancelled.
 */
static int ask_model(
    const ai_provider_info_t *provider,
    const char *current_model,
    char *model,
    size_t size
)
{
    char answer[AI_MODEL_MAX];

    /*
     * The default: the current model, or the default model of
     * the provider when there is no current one.
     */
    const char *fallback = current_model[0] != '\0'
        ? current_model
        : provider->default_model;

    /*
     * Number of suggested models.
     */
    size_t count = 0;

    printf(
        "\nStep 2 of 4: choose the model of %s\n\n",
        display_name(provider)
    );

    for (; provider->models[count].name != NULL; count++) {
        printf(
            "  %zu) %-24s %s\n",
            count + 1,
            provider->models[count].name,
            provider->models[count].note
        );
    }

    printf(
        "\nType a number, or the name of another model of %s.\n",
        display_name(provider)
    );

    for (;;) {
        printf("Model [%s]: ", fallback);

        if (read_answer(answer, sizeof(answer)) != 0) {
            return -1;
        }

        /*
         * Enter: the default.
         */
        if (answer[0] == '\0') {
            snprintf(model, size, "%s", fallback);
            return 0;
        }

        /*
         * A number from the list.
         */
        size_t choice = parse_choice(answer, count);

        if (choice != 0) {
            snprintf(model, size, "%s", provider->models[choice - 1].name);
            return 0;
        }

        /*
         * Only digits, but not a number of the list: more
         * likely a slip than the name of a model.
         */
        if (answer[strspn(answer, "0123456789")] == '\0') {
            printf(
                "There is no model number %s. Type a number from "
                "1 to %zu, or a model name.\n",
                answer,
                count
            );
            continue;
        }

        /*
         * Anything else is taken as a model name. It is not
         * checked here: providers add models all the time. The
         * test in step 4 shows whether the name is right.
         */
        snprintf(model, size, "%s", answer);
        return 0;
    }
}

/*
 * Tells whether an API key is stored in the keyring under the
 * given name. The key is read only to learn that it exists and
 * is wiped at once.
 */
static bool key_is_stored(const char *key_name)
{
    char key[AI_KEY_MAX];

    bool stored = keyring_load(key_name, key, sizeof(key), NULL) == 0;

    explicit_bzero(key, sizeof(key));

    return stored;
}

/*
 * Step 3: the API key.
 *
 * Reads the key without showing it and stores it in the keyring
 * of the system under key_name. When a key is already stored
 * there, Enter keeps it.
 *
 * Returns:
 *    0  - a key is stored (a new one, or the old one was kept)
 *   -1  - the user cancelled, or gave no key and none is stored
 *   -2  - the key could not be stored (already reported)
 */
static int ask_key(
    const ai_provider_info_t *provider,
    const char *key_name
)
{
    char key[AI_KEY_MAX];
    char error_text[KEYRING_ERROR_TEXT_MAX];

    bool already_stored = key_is_stored(key_name);

    printf(
        "\nStep 3 of 4: the API key of %s\n\n"
        "The key is kept in the keyring of your system, not in\n"
        "a file. It is not shown while you type or paste it.\n",
        display_name(provider)
    );

    if (already_stored) {
        printf(
            "A key is already stored. Press Enter to keep it, or "
            "type a new one.\n\n"
            "API key [keep the stored key]: "
        );
    } else {
        printf("\nAPI key: ");
    }

    if (read_hidden_answer(key, sizeof(key)) != 0) {
        explicit_bzero(key, sizeof(key));
        return -1;
    }

    /*
     * Enter without a key.
     */
    if (key[0] == '\0') {
        return already_stored ? 0 : -1;
    }

    int result = keyring_store(key_name, key, error_text);
    int saved_errno = errno;

    /*
     * The key is in the keyring now, or could not be stored.
     * Either way it must not stay in memory.
     */
    explicit_bzero(key, sizeof(key));

    if (result != 0) {
        fprintf(
            stderr,
            "The key could not be stored in the keyring: %s\n",
            error_text[0] != '\0' ? error_text : strerror(saved_errno)
        );
        return -2;
    }

    printf("The key was stored in the keyring.\n");

    return 0;
}

/*
 * Step 4: test it?
 *
 * Returns 1 for yes, 0 for no and -1 when the user cancelled.
 * Enter means yes.
 */
static int ask_test(void)
{
    char answer[8];

    printf(
        "\nStep 4 of 4: test the configuration\n\n"
        "A short message can be sent to check that the provider,\n"
        "the model and the key work together. It uses a handful\n"
        "of tokens.\n\n"
    );

    for (;;) {
        printf("Send a test message? [Y/n]: ");

        if (read_answer(answer, sizeof(answer)) != 0) {
            return -1;
        }

        if (answer[0] == '\0' || answer[0] == 'y' || answer[0] == 'Y') {
            return 1;
        }

        if (answer[0] == 'n' || answer[0] == 'N') {
            return 0;
        }

        printf("Type y for yes or n for no.\n");
    }
}

/*
 * Explains a failed test in plain words: what went wrong and
 * what to do about it.
 *
 * status is the result of ai_send() and error_text the
 * explanation that came with it.
 */
static void explain_test_failure(
    ai_status_t status,
    const char *error_text,
    const char *model
)
{
    switch (status) {

    case AI_ERROR_CANCELLED:
        printf("\nThe test was cancelled.\n");
        break;

    case AI_ERROR_AUTH:
        printf(
            "The provider did not accept the API key.\n"
            "Run this command again and type the key once more.\n"
        );
        break;

    case AI_ERROR_REQUEST:
        printf(
            "The provider did not accept the request. Most often "
            "the model name is wrong: \"%s\".\n"
            "Run this command again and choose a model from "
            "the list.\n",
            model
        );
        break;

    case AI_ERROR_NETWORK:
        printf(
            "The provider could not be reached. Check the "
            "internet connection and try again.\n"
        );
        break;

    case AI_ERROR_RATE_LIMIT:
        printf(
            "The provider is limiting requests for this key right "
            "now, or its quota is used up. The configuration may "
            "still be right; try again later.\n"
        );
        break;

    case AI_ERROR_SERVER:
        printf(
            "The provider has a problem at the moment. The "
            "configuration may still be right; try again later.\n"
        );
        break;

    default:
        printf("The test did not succeed.\n");
        break;
    }

    /*
     * The exact words of the provider, for those who want them.
     */
    if (status != AI_ERROR_CANCELLED && error_text[0] != '\0') {
        printf("Details: %s\n", error_text);
    }
}

/*
 * Sends the test message with the saved configuration.
 *
 * Returns 0 when the model answered and -1 otherwise. The
 * outcome is explained to the user in both cases.
 */
static int run_test(ai_settings_t *settings)
{
    char error_text[AI_ERROR_TEXT_MAX];
    ai_reply_t reply;

    /*
     * Fetch the key that step 3 put into the keyring.
     */
    if (ai_settings_load_key(settings, error_text) != 0) {
        printf(
            "The API key could not be read back from the keyring"
            "%s%s\n",
            error_text[0] != '\0' ? ": " : ".",
            error_text
        );
        return -1;
    }

    ai_conversation_t *conversation = ai_conversation_new();

    if (conversation == NULL) {
        printf("Out of memory.\n");
        ai_settings_wipe(settings);
        return -1;
    }

    /*
     * The test uses a copy of the configuration with a small
     * reply limit and without the system prompt, so it is quick
     * and cheap whatever the user has configured.
     */
    static ai_config_t test_config;

    test_config = settings->config;
    test_config.max_tokens = TEST_MAX_TOKENS;
    test_config.system_prompt[0] = '\0';

    printf("\nSending a test message (press Ctrl+C to cancel)...\n");
    fflush(stdout);

    /*
     * Forget a Ctrl+C that was pressed earlier.
     */
    interrupted = 0;

    ai_status_t status = ai_send(
        &test_config,
        conversation,
        TEST_MESSAGE,
        &interrupted,
        &reply,
        error_text
    );

    /*
     * The key is not needed any more.
     */
    explicit_bzero(&test_config, sizeof(test_config));
    ai_settings_wipe(settings);
    ai_conversation_free(conversation);

    if (status != AI_OK) {
        explain_test_failure(status, error_text, settings->config.model);
        return -1;
    }

    printf(
        "\nThe configuration works: %s answered.\n",
        reply.model[0] != '\0' ? reply.model : settings->config.model
    );

    ai_reply_free(&reply);

    return 0;
}

int command_ai_config(void)
{
    /*
     * static: the configuration with its buffers is large.
     */
    static ai_settings_t settings;

    if (ai_settings_load(&settings) != 0) {
        return 1;
    }

    /*
     * Handle Ctrl+C (SIGINT), see handle_signal().
     */
    struct sigaction sa = {0};

    sa.sa_handler = handle_signal;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = 0;

    if (sigaction(SIGINT, &sa, NULL) != 0) {
        perror("sigaction");
        return 1;
    }

    printf(
        "AI configuration\n"
        "\n"
        "Answer four questions to set up the AI integration.\n"
        "Press Enter to accept the answer shown in [brackets].\n"
        "Press Ctrl+C to cancel.\n"
        "\n"
    );

    /*
     * Step 1: the provider.
     */
    const ai_provider_info_t *provider = ask_provider(&settings);

    if (provider == NULL) {
        printf("Cancelled. Nothing was changed.\n");
        return 0;
    }

    /*
     * The answers are collected in a copy. The copy replaces
     * the configuration only when it is saved.
     */
    static ai_settings_t changed;

    changed = settings;

    bool provider_changed =
        strcmp(provider->name, settings.config.provider) != 0;

    /*
     * A new provider resets what belonged to the old one: the
     * model, the key name and a custom address.
     */
    if (provider_changed) {
        ai_settings_set_provider(&changed, provider->name);
    }

    /*
     * Step 2: the model.
     */
    if (ask_model(
            provider,
            provider_changed ? "" : settings.config.model,
            changed.config.model,
            sizeof(changed.config.model)) != 0) {

        printf("Cancelled. Nothing was changed.\n");
        return 0;
    }

    /*
     * Step 3: the API key, for providers that need one.
     */
    if (provider->requires_key) {
        int result = ask_key(provider, changed.key_name);

        if (result == -1) {
            printf("No key given. Nothing was changed.\n");
            return 0;
        }

        if (result != 0) {
            return 1;
        }
    }

    /*
     * The provider, the model and the key are known: save.
     */
    if (ai_settings_save(&changed) != 0) {
        return 1;
    }

    settings = changed;

    printf(
        "\nSaved: provider %s, model %s.\n",
        display_name(provider),
        settings.config.model
    );

    /*
     * Step 4: the optional test.
     */
    int test = ask_test();

    if (test <= 0) {
        printf(
            "\nThe configuration was saved. No test was run, so it "
            "is not known yet whether it works.\n"
        );
        return 0;
    }

    if (run_test(&settings) != 0) {
        printf(
            "\nThe configuration was saved, but the test did not "
            "succeed.\n"
        );
        return 1;
    }

    return 0;
}
