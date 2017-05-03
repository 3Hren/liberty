#pragma once

#include "liberty.h"

namespace liberty {

class http_request {
    liberty_http_request* request;

public:
    http_request() :
        request(liberty_http_request_make())
    {}

    ~http_request() {
        liberty_http_request_free(request);
    }

    auto set_post() -> void {
        liberty_http_request_post(request);
    }

    auto set_request_uri(const std::string& uri) -> void {
        liberty_http_request_url(request, uri.data(), uri.size());
    }

    auto set_data(const std::string& data) -> void {
        liberty_http_request_data(request, data.data(), data.size());
    }

    template<typename F>
    auto set_complete_callback(F fn) -> void {
        std::unique_ptr<F> holder(new F(std::move(fn)));
        liberty_http_request_complete_callback(request, [](const liberty_http_error* error, const liberty_http_response* response, void* data) {
            std::unique_ptr<F> holder(static_cast<F*>(data));
            (*holder)(error, response);
        }, holder.release());
    }

    auto into_inner() && -> liberty_http_request* {
        auto result = request;
        request = nullptr;
        return result;
    }
};

class http_client {
    liberty_http_client* client;

public:
    http_client() :
        client(liberty_http_client_make())
    {}

    ~http_client() {
        liberty_http_client_free(client);
    }

    auto perform(http_request&& request) -> void {
        liberty_http_client_perform(client, std::move(request).into_inner());
    }
};

} // namespace liberty
