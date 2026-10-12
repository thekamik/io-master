/*
 * HTTP transport for the AI providers, built on libcurl.
 *
 * libcurl is used through its "easy" interface: a handle is
 * created, options are set on it, and curl_easy_perform() runs
 * the whole request. While it runs, libcurl calls two functions
 * of this file:
 *
 *     collect_body()    with every piece of the answer
 *     check_cancel()    regularly, to ask whether to go on
 */

#include "ai/http.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

#include <curl/curl.h>

/*
 * Longest answer that is accepted, in bytes. A reply of an AI
 * model is far smaller; the limit only stops a broken or hostile
 * server from filling the memory.
 */
#define RESPONSE_SIZE_LIMIT (32u * 1024u * 1024u)

/*
 * Longest time to wait for the connection itself, in seconds.
 */
#define CONNECT_TIMEOUT_SECONDS 30L

/*
 * Name sent to the server in the User-Agent header.
 */
#define USER_AGENT "io-master"

/*
 * Answer body that grows as the pieces arrive.
 */
typedef struct {
    char *data;
    size_t size;

    /*
     * true when the answer exceeded RESPONSE_SIZE_LIMIT or
     * memory ran out. The transfer is stopped in that case.
     */
    bool failed;
} body_buffer_t;

/*
 * Called by libcurl with every piece of the answer.
 *
 * Returns the number of bytes handled. Any other value tells
 * libcurl to stop the transfer.
 */
static size_t collect_body(
    char *piece,
    size_t unit_size,
    size_t unit_count,
    void *userdata
)
{
    body_buffer_t *buffer = userdata;
    size_t piece_size = unit_size * unit_count;

    if (buffer->size + piece_size > RESPONSE_SIZE_LIMIT) {
        buffer->failed = true;
        return 0;
    }

    /*
     * One extra byte for the '\0' that ends the text.
     */
    char *grown = realloc(buffer->data, buffer->size + piece_size + 1);

    if (grown == NULL) {
        buffer->failed = true;
        return 0;
    }

    memcpy(grown + buffer->size, piece, piece_size);

    buffer->data = grown;
    buffer->size += piece_size;
    buffer->data[buffer->size] = '\0';

    return piece_size;
}

/*
 * Called by libcurl about once a second, and more often while
 * data moves. A non-zero return value aborts the request.
 */
static int check_cancel(
    void *userdata,
    curl_off_t download_total,
    curl_off_t download_now,
    curl_off_t upload_total,
    curl_off_t upload_now
)
{
    (void)download_total; // Unused parameter
    (void)download_now;   // Unused parameter
    (void)upload_total;   // Unused parameter
    (void)upload_now;     // Unused parameter

    const volatile sig_atomic_t *cancel = userdata;

    return (cancel != NULL && *cancel != 0) ? 1 : 0;
}

/*
 * Translates the result of curl_easy_perform() to one of the
 * errno values documented in http.h.
 */
static int curl_code_to_errno(CURLcode code)
{
    switch (code) {

    case CURLE_ABORTED_BY_CALLBACK:
        return ECANCELED;

    case CURLE_OPERATION_TIMEDOUT:
        return ETIMEDOUT;

    /*
     * The TLS handshake failed (for example the server offers
     * only TLS 1.2), or the certificate is not trusted.
     */
    case CURLE_SSL_CONNECT_ERROR:
    case CURLE_PEER_FAILED_VERIFICATION:
    case CURLE_SSL_CACERT_BADFILE:
    case CURLE_SSL_CERTPROBLEM:
    case CURLE_SSL_CIPHER:
        return EPROTO;

    default:
        return EIO;
    }
}

/*
 * Stores an explanation in error_text, when the caller wants one.
 */
static void set_error_text(char *error_text, const char *text)
{
    if (error_text != NULL) {
        snprintf(error_text, HTTP_ERROR_TEXT_MAX, "%s", text);
    }
}

/*
 * Prepares libcurl once per program run.
 *
 * Returns 0 on success and -1 on error.
 */
static int initialize_curl(void)
{
    static bool initialized = false;

    if (!initialized) {

        if (curl_global_init(CURL_GLOBAL_DEFAULT) != CURLE_OK) {
            return -1;
        }

        initialized = true;
    }

    return 0;
}

int http_post(
    const http_request_t *request,
    http_response_t *response,
    char *error_text
)
{
    /*
     * Buffer in which libcurl describes a failure.
     */
    char curl_error[CURL_ERROR_SIZE] = "";

    body_buffer_t buffer = { NULL, 0, false };

    if (error_text != NULL) {
        error_text[0] = '\0';
    }

    if (request == NULL ||
        response == NULL ||
        request->url == NULL ||
        request->body == NULL) {

        set_error_text(error_text, "incomplete request");
        errno = EINVAL;
        return -1;
    }

    response->status = 0;
    response->body = NULL;
    response->body_size = 0;

    /*
     * A secret is never sent anywhere but to an https:// address.
     * This is checked here as well as enforced by libcurl below,
     * so the reason is reported clearly.
     */
    if (request->secure &&
        strncasecmp(request->url, "https://", strlen("https://")) != 0) {

        set_error_text(
            error_text,
            "the API key is only sent over HTTPS, "
            "but the address does not start with https://"
        );
        errno = EPROTOTYPE;
        return -1;
    }

    if (initialize_curl() != 0) {
        set_error_text(error_text, "cannot initialize libcurl");
        errno = EIO;
        return -1;
    }

    CURL *handle = curl_easy_init();

    if (handle == NULL) {
        set_error_text(error_text, "cannot create a connection");
        errno = EIO;
        return -1;
    }

    /*
     * Build the list of header lines.
     */
    struct curl_slist *headers = NULL;

    for (size_t i = 0;
         request->headers != NULL && request->headers[i] != NULL;
         i++) {

        struct curl_slist *grown = curl_slist_append(
            headers,
            request->headers[i]
        );

        if (grown == NULL) {
            curl_slist_free_all(headers);
            curl_easy_cleanup(handle);
            set_error_text(error_text, "out of memory");
            errno = EIO;
            return -1;
        }

        headers = grown;
    }

    /*
     * What to send.
     */
    curl_easy_setopt(handle, CURLOPT_URL, request->url);
    curl_easy_setopt(handle, CURLOPT_POSTFIELDS, request->body);
    curl_easy_setopt(handle, CURLOPT_HTTPHEADER, headers);
    curl_easy_setopt(handle, CURLOPT_USERAGENT, USER_AGENT);

    /*
     * Where the answer goes.
     */
    curl_easy_setopt(handle, CURLOPT_WRITEFUNCTION, collect_body);
    curl_easy_setopt(handle, CURLOPT_WRITEDATA, &buffer);
    curl_easy_setopt(handle, CURLOPT_ERRORBUFFER, curl_error);

    /*
     * Time limits. CURLOPT_NOSIGNAL keeps libcurl from using
     * signals for its timeouts, which would disturb the SIGINT
     * handling of the shell.
     */
    curl_easy_setopt(handle, CURLOPT_TIMEOUT, request->timeout_seconds);
    curl_easy_setopt(
        handle,
        CURLOPT_CONNECTTIMEOUT,
        CONNECT_TIMEOUT_SECONDS
    );
    curl_easy_setopt(handle, CURLOPT_NOSIGNAL, 1L);

    /*
     * Cancellation: let libcurl call check_cancel().
     */
    curl_easy_setopt(handle, CURLOPT_XFERINFOFUNCTION, check_cancel);
    curl_easy_setopt(
        handle,
        CURLOPT_XFERINFODATA,
        (void *)request->cancel
    );
    curl_easy_setopt(handle, CURLOPT_NOPROGRESS, 0L);

    /*
     * Security.
     *
     * Redirects are never followed. A redirect would make
     * libcurl repeat the request, with its headers, at an
     * address chosen by the server.
     */
    curl_easy_setopt(handle, CURLOPT_FOLLOWLOCATION, 0L);

    if (request->secure) {
        /*
         * A request with a secret: HTTPS only, TLS 1.3 or newer.
         */
        curl_easy_setopt(handle, CURLOPT_PROTOCOLS_STR, "https");
        curl_easy_setopt(
            handle,
            CURLOPT_SSLVERSION,
            (long)CURL_SSLVERSION_TLSv1_3
        );
    } else {
        /*
         * No secret: plain HTTP is allowed as well.
         */
        curl_easy_setopt(handle, CURLOPT_PROTOCOLS_STR, "http,https");
    }

    /*
     * Certificate checks are on by default. They are set
     * explicitly, so that the intent is visible and a changed
     * default cannot turn them off.
     */
    curl_easy_setopt(handle, CURLOPT_SSL_VERIFYPEER, 1L);
    curl_easy_setopt(handle, CURLOPT_SSL_VERIFYHOST, 2L);

    if (request->ca_file != NULL && request->ca_file[0] != '\0') {
        curl_easy_setopt(handle, CURLOPT_CAINFO, request->ca_file);
    }

    /*
     * Run the request. This blocks until the answer is complete,
     * the time is up or check_cancel() aborts it.
     */
    CURLcode code = curl_easy_perform(handle);

    long status = 0;

    if (code == CURLE_OK) {
        curl_easy_getinfo(handle, CURLINFO_RESPONSE_CODE, &status);
    }

    curl_slist_free_all(headers);
    curl_easy_cleanup(handle);

    if (code != CURLE_OK) {
        int error_number = curl_code_to_errno(code);

        /*
         * collect_body() stopped the transfer itself.
         */
        if (buffer.failed) {
            set_error_text(error_text, "the answer is too large");
            error_number = EIO;
        } else if (error_number == ECANCELED) {
            set_error_text(error_text, "cancelled");
        } else {
            /*
             * Prefer the detailed text of libcurl.
             */
            set_error_text(
                error_text,
                curl_error[0] != '\0'
                    ? curl_error
                    : curl_easy_strerror(code)
            );
        }

        free(buffer.data);

        errno = error_number;
        return -1;
    }

    /*
     * An answer without a body still gets an empty text.
     */
    if (buffer.data == NULL) {
        buffer.data = calloc(1, 1);

        if (buffer.data == NULL) {
            set_error_text(error_text, "out of memory");
            errno = EIO;
            return -1;
        }
    }

    response->status = status;
    response->body = buffer.data;
    response->body_size = buffer.size;

    return 0;
}

void http_response_free(http_response_t *response)
{
    if (response == NULL) {
        return;
    }

    free(response->body);

    response->body = NULL;
    response->body_size = 0;
}
