import rrr.request_options;
import rrr.inline_consumer;

int main() {
    return rrr::crate_draw() == 0.7 && rrr::inline_draw() == 0.7 ? 0 : 1;
}
