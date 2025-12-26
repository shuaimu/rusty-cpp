#include <memory>
#include <list>

struct Request { int data; };
void handler_fn(std::unique_ptr<Request> req);
void create_coro(auto fn);

// @safe
void test() {
    std::list<std::unique_ptr<Request>> complete_requests;
    
    // @unsafe
    {
        // First loop: collect requests
        for (;;) {
            int packet_size = 10;
            
            auto req = std::make_unique<Request>();
            req->data = packet_size;
            complete_requests.push_back(std::move(req));
            
            if (packet_size < 0) break;
        }
        
        // Second loop: process requests  
        while (!complete_requests.empty()) {
            std::unique_ptr<Request> req = std::move(complete_requests.front());
            complete_requests.pop_front();
            
            std::unique_ptr<Request> req_owned = std::move(req);
            create_coro([req_owned = std::move(req_owned)]() mutable {
                handler_fn(std::move(req_owned));
            });
        }
    }
}
