#pragma once

#ifdef __cplusplus
extern "C" {
#endif

struct liberty_http_error;
struct liberty_http_client;
struct liberty_http_request;
struct liberty_http_response;

typedef void (*complete_callback)(const liberty_http_error *, const liberty_http_response *, void *);

const char *liberty_error_extra(const liberty_http_error *error);
size_t liberty_error_extra_size(const liberty_http_error *error);

liberty_http_request *liberty_http_request_make(void);
void liberty_http_request_free(liberty_http_request *request);
int liberty_http_request_get(liberty_http_request *request);

/// Tells the library to do a regular HTTP post.
int liberty_http_request_post(liberty_http_request *request);
int liberty_http_request_url(liberty_http_request *request, const char *data, size_t size);
int liberty_http_request_data(liberty_http_request *request, const char *data, size_t size);
void liberty_http_request_complete_callback(liberty_http_request *request, complete_callback, void *data);

int liberty_http_response_code(const liberty_http_response *response);
const char *liberty_http_response_body(const liberty_http_response *response);
size_t liberty_http_response_body_size(const liberty_http_response *response);

liberty_http_client *liberty_http_client_make(void);
void liberty_http_client_free(liberty_http_client *client);
void liberty_http_client_perform(liberty_http_client *client, liberty_http_request *request);

#ifdef __cplusplus
}
#endif
