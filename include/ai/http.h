/*
 * HTTP transport for the AI providers.
 *
 * Sends one JSON document with POST and returns the answer.
 * Built on libcurl. The module knows nothing about AI: it only
 * moves bytes, and enforces the transport security rules.
 *
 * Security rules
 * --------------
 * A request marked as secure (http_request_t.secure) carries
 * a secret, the API key. For such a request:
 *
 *   - only https:// URLs are accepted,
 *   - the connection has to use TLS 1.3 or newer,
 *   - the certificate of the server is always verified,
 *   - redirects are never followed, so the key cannot be passed
 *     on to another address.
 *
 * A request that is not secure carries no secret. It may also
 * use plain http://, which local AI servers usually speak.
 */

#ifndef AI_HTTP_H
#define AI_HTTP_H

#include <signal.h>
#include <stdbool.h>
#include <stddef.h>

/*
 * Size of the buffer that receives the explanation of
 * a failed request.
 */
#define HTTP_ERROR_TEXT_MAX 256

/*
 * One POST request.
 */
typedef struct {
    /*
     * Full address, for example
     * "https://api.anthropic.com/v1/messages".
     */
    const char *url;

    /*
     * Header lines in the form "Name: value". The array ends
     * with NULL.
     */
    const char *const *headers;

    /*
     * Body of the request: a JSON document as text.
     */
    const char *body;

    /*
     * true when the request carries a secret. Turns on the
     * security rules described at the top of this file.
     */
    bool secure;

    /*
     * Optional file with the certificate authorities to trust
     * (PEM format). It replaces the certificates of the system
     * for this request. NULL or "" uses the certificates of the
     * system. Needed for servers with a private certificate.
     */
    const char *ca_file;

    /*
     * Longest time the whole request may take, in seconds.
     */
    long timeout_seconds;

    /*
     * Lets the caller abort the request: it is given up soon
     * after *cancel becomes non-zero. The shell passes the flag
     * set by Ctrl+C. May be NULL.
     */
    const volatile sig_atomic_t *cancel;
} http_request_t;

/*
 * The answer of the server.
 */
typedef struct {
    /*
     * HTTP status code, for example 200 or 401.
     */
    long status;

    /*
     * Body of the answer as text, always terminated with '\0'.
     * Allocated by http_post(); release it with
     * http_response_free().
     */
    char *body;
    size_t body_size;
} http_response_t;

/*
 * Sends the request and waits for the answer.
 *
 * An answer with an error status (4xx, 5xx) is still a success
 * of this function: the caller looks at response->status.
 *
 * error_text is a buffer of HTTP_ERROR_TEXT_MAX characters that
 * receives an explanation on failure. May be NULL.
 *
 * Returns:
 *   0  - an answer was received
 *  -1  - no answer; errno is one of:
 *          EINVAL      missing URL or body
 *          EPROTOTYPE  a secure request with a URL that is not
 *                      https://
 *          ECANCELED   aborted through cancel
 *          ETIMEDOUT   the server did not answer in time
 *          EPROTO      TLS problem: version below 1.3 or
 *                      a certificate that cannot be trusted
 *          EIO         any other network problem
 */
int http_post(
    const http_request_t *request,
    http_response_t *response,
    char *error_text
);

/*
 * Releases the body of an answer. Safe to call on an answer
 * that was never filled, as long as it was zero-initialized.
 */
void http_response_free(http_response_t *response);

#endif
