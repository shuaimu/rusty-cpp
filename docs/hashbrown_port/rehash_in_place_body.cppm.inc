void RawTableInner::rehash_in_place(const std::function<uint64_t(RawTableInner&, size_t)>& hasher, size_t size_of, rusty::Option<rusty::UnsafeFn<void(uint8_t*)>> drop) {
    // @unsafe
    {
        this->prepare_rehash_in_place();
    }
    // PORT FIX: operate on *this DIRECTLY. The transpiler lowered Rust's `guard(&mut self, ...)` to a
    // by-value `guard((*this), ...)`, which copies the table so the in-place rehash's growth_left/items
    // updates never reach the real table -> growth_left stays stale (0) and later inserts underflow it
    // -> the probe runs past the end of the table. Rust rehashes in place through the &mut borrow; we
    // do the same here on *this. The Rust `guard(...)` closure is panic cleanup only (run on unwind),
    // so it is preserved as a catch handler rather than a by-value ScopeGuard copy.
    auto _cleanup = [this, &drop, size_of]() mutable {
        for (auto&& i : rusty::for_in(rusty::range(0, this->num_buckets()))) {
            // @unsafe
            {
                if (rusty::detail::deref_if_pointer_like(this->ctrl(std::move(i))) == rusty::clone(Tag::DELETED)) {
                    this->set_ctrl(std::move(i), rusty::clone(rusty::clone(Tag::EMPTY)));
                    if (drop.is_some()) {
                        auto&& _iflet_bound_scrutinee = drop;
                        decltype(auto) drop = _iflet_bound_scrutinee.unwrap();
                        drop.call_unsafe(this->bucket_ptr(std::move(i), std::move(size_of)));
                    }
                    this->items -= 1;
                }
            }
        }
        this->growth_left = bucket_mask_to_capacity(this->bucket_mask) - rusty::detail::deref_if_pointer_like(this->items);
    };
    try {
    for (auto&& i : rusty::for_in(rusty::range(0, this->num_buckets()))) {
        // @unsafe
        {
            if (rusty::detail::deref_if_pointer_like(this->ctrl(std::move(i))) != rusty::clone(Tag::DELETED)) {
                continue;
            }
        }
        const auto i_p = this->bucket_ptr(std::move(i), std::move(size_of));
        while (true) {
            auto hash = hasher((*this), std::move(i));
            auto new_i = this->find_insert_index(std::move(hash));
            if (likely(this->is_in_same_group(std::move(i), std::move(new_i), std::move(hash)))) {
                // @unsafe
                {
                    this->set_ctrl_hash(std::move(i), std::move(hash));
                }
                break; // Rust `continue 'outer`: advance the outer for-loop, not the inner while(true). Transpiler drops loop labels (emit_expr.rs Continue/Break ignore the label).
            }
            const auto new_i_p = this->bucket_ptr(std::move(new_i), std::move(size_of));
            const auto prev_ctrl = this->replace_ctrl_hash(std::move(new_i), std::move(hash));
            if (rusty::detail::deref_if_pointer_like(prev_ctrl) == rusty::clone(Tag::EMPTY)) {
                // @unsafe
                {
                    this->set_ctrl(std::move(i), rusty::clone(rusty::clone(Tag::EMPTY)));
                }
                // @unsafe
                {
                    rusty::ptr::copy_nonoverlapping(std::move(i_p), std::move(new_i_p), std::move(size_of));
                }
                break; // Rust `continue 'outer`: advance the outer for-loop, not the inner while(true). Transpiler drops loop labels (emit_expr.rs Continue/Break ignore the label).
            }
        assert(true);  // assert((prev_ctrl == Tag :: DELETED));...
            // @unsafe
            {
                rusty::ptr::swap_nonoverlapping(std::move(i_p), std::move(new_i_p), std::move(size_of));
            }
        }
    }
    } catch (...) { _cleanup(); throw; }
    this->growth_left = bucket_mask_to_capacity(this->bucket_mask) - rusty::detail::deref_if_pointer_like(this->items);
}
