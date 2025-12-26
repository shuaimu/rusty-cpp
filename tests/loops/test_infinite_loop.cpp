#include <memory>
#include <list>

struct Request { int data; };

// @safe
void test(std::list<std::unique_ptr<Request>>& complete_requests) {
    // @unsafe
    {
        for (;;) {
            int packet_size = 10;
            
            auto req = std::make_unique<Request>();
            req->data = packet_size;
            
            // Move into list
            complete_requests.push_back(std::move(req));
            
            if (packet_size < 0) {
                break;  // Exit condition
            }
        }
    }
}
