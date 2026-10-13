/*
 * The "ai-config" command: a short series of questions that sets
 * up the AI integration of the program.
 *
 * It asks for the provider, the model and the API key, saves the
 * answers and offers to test them with one short message. The
 * chat of the "ai" protocol and, later, the device protocols
 * only use the result.
 *
 * The answers are stored in their own file,
 * im_config/ai_config.json (see ai/ai_settings.h). The API key
 * goes to the keyring of the system instead.
 */

#ifndef AI_CONFIG_H
#define AI_CONFIG_H

/*
 * Names of the command on the command line: the full name and
 * its short form.
 */
#define AI_CONFIG_COMMAND "ai-config"
#define AI_CONFIG_COMMAND_SHORT "aic"

/*
 * Asks the questions, saves the configuration and optionally
 * tests it.
 *
 * Reports its own errors on stderr and returns the exit code
 * of the program:
 *   0  - the configuration was saved (and, when the user asked
 *        for the test, the test passed), or the user cancelled
 *   1  - error, or the test failed
 */
int command_ai_config(void);

#endif
