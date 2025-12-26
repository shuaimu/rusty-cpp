#include <memory>
#include <list>
#include <unordered_map>
#include <functional>

struct Marshal {
    int content_size() const { return 100; }
    template<typename T>
    Marshal& operator>>(T& t) { return *this; }
    size_t read_from_marshal(Marshal& m, int size) { return size; }
};

struct Request { 
    Marshal m;
    int xid;
};

using WeakConn = int;
using RequestHandler = std::function<void(std::unique_ptr<Request>, WeakConn)>;

class ServerConnection {
    std::unordered_map<int, RequestHandler> handlers_;
    WeakConn weak_self_;
    Marshal in_;
    
    void reply(Request& req, int code) {}
    void CreateRun(auto fn) {}
    
public:
    // @safe
    bool handle_read() {
        std::list<std::unique_ptr<Request>> complete_requests;
        
        // @unsafe
        {
            // First loop: collect requests (same variable name req)
            for (;;) {
                int packet_size = 10;
                
                // Same variable name as the second loop
                auto req = std::make_unique<Request>();
                req->m.read_from_marshal(in_, packet_size);
                req->xid = packet_size;
                complete_requests.push_back(std::move(req));
                
                if (packet_size < 0) break;
            }
            
            // Second loop: process requests (same variable name req)
            while (!complete_requests.empty()) {
                std::unique_ptr<Request> req = std::move(complete_requests.front());
                complete_requests.pop_front();
                
                if (req->m.content_size() < 4) {
                    reply(*req, -1);
                } else {
                    int rpc_id;
                    req->m >> rpc_id;
                    
                    auto it = handlers_.find(rpc_id);
                    if (it == handlers_.end()) {
                        reply(*req, -2);
                    } else {
                        auto weak_this = weak_self_;
                        auto handler = it->second;
                        std::unique_ptr<Request> req_owned = std::move(req);
                        CreateRun([handler, req_owned = std::move(req_owned), weak_this]() mutable {
                            handler(std::move(req_owned), weak_this);
                        });
                    }
                }
            }
        }
        return false;
    }
};
