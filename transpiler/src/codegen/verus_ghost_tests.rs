//! Codegen fixtures for `--verus-exec` ghost-residue lowering: Verus-erased
//! source goes through `crate::verus_lower` (stage 2) and then codegen, the
//! way `read_crate_source_units` hands it over in crate mode.
use super::*;

fn translate_lowered(source: &str) -> String {
    let identity = std::path::PathBuf::from("src/lib.rs");
    let mut units = vec![crate::verus_lower::LowerUnit {
        identity: &identity,
        original: source,
        prepared: source.to_string(),
    }];
    crate::verus_lower::lower_crate("demo", &mut units).expect("lowering succeeds");
    let mut generator = CodeGen::new();
    generator.emit_file(&syn::parse_str(&units[0].prepared).unwrap(), None);
    generator.into_output()
}

/// Lion's reactor shapes after `EraseAll`: a ghost log field, a derived
/// struct holding a ghost index, ghost tuple elements and ghost locals.
const REACTOR_SHAPES: &str = r#"
use vstd::prelude::*;
pub type InstantView = nat;
pub type Log = Seq<u64>;
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TimerEntry {
    pub deadline: u64,
    pub log_index: Ghost<int>,
}
impl View for TimerEntry {
    type V = (InstantView, nat, int);
}
pub struct Reactor {
    pub log: Ghost<Log>,
    pub next: u64,
}
impl Reactor {
    pub fn new() -> Self {
        let result = Reactor { log: Ghost::assume_new_fallback(|| unreachable!()), next: 0 };
        {}
        result
    }
    pub fn entry(&self, deadline: u64) -> TimerEntry {
        let log_index: Ghost<int> = Ghost::assume_new_fallback(|| unreachable!());
        TimerEntry { deadline, log_index }
    }
    pub fn pop(&mut self) -> Option<(u64, Ghost<int>, Ghost<InstantView>)> {
        let log_idx: Ghost<int> = Ghost::assume_new_fallback(|| unreachable!());
        let deadline: Ghost<InstantView> = Ghost::assume_new_fallback(|| unreachable!());
        {};
        Some((self.next, log_idx, deadline))
    }
}
"#;

#[test]
fn ghost_state_lowers_to_the_empty_tag_in_every_position() {
    let cpp = translate_lowered(REACTOR_SHAPES);
    // Fields: the tag, with no storage.
    assert!(cpp.contains("[[no_unique_address]] rusty::Ghost log_index;"), "{cpp}");
    assert!(cpp.contains("[[no_unique_address]] rusty::Ghost log;"), "{cpp}");
    // Values: a constructed tag.
    assert!(cpp.contains(".log = rusty::Ghost{}"), "{cpp}");
    assert!(cpp.contains("rusty::Ghost log_index = rusty::Ghost{};"), "{cpp}");
    // Tuple arity is kept: two tags, no element dropped.
    assert!(
        cpp.contains("rusty::Option<std::tuple<uint64_t, rusty::Ghost, rusty::Ghost>>"),
        "{cpp}"
    );
    // Derives go through the tag's own clone/==/debug.
    assert!(cpp.contains("rusty::clone(this->log_index)"), "{cpp}");
    assert!(cpp.contains("bool operator==(const TimerEntry&) const = default;"), "{cpp}");
    assert!(cpp.contains("rusty::to_debug_string(this->log_index)"), "{cpp}");
    // Nothing of the spec side survives in the crate's own code (the output
    // also carries the runtime preamble, e.g. a serde `Token_Seq`): no spec
    // types, views, vstd, ghost payload types, the reserved Rust spelling of
    // the marker, or a hand-attention slot.
    // `derive(Copy)` is left out of the check: codegen marks every
    // `derive(Copy)` with a TODO today, ghost field or not (a general gap).
    let user = cpp[cpp.find("struct TimerEntry").expect("TimerEntry is emitted")..]
        .replace("// TODO: derive(Copy)", "");
    for residue in [
        "vstd", "View", "Seq", "nat", "int_", "InstantView", "Log", "Ghost<", "RustyVerusGhost",
        "TODO",
    ] {
        assert!(!user.contains(residue), "`{residue}` survives:\n{user}");
    }
}

#[test]
fn vec_set_lowers_to_index_assignment_in_cpp() {
    let cpp = translate_lowered(
        r#"
use vstd::prelude::*;
pub struct Slab<V: View> {
    pub inner: Vec<Option<V>>,
    pub offset: u64,
}
impl<V: View> View for Slab<V> {
    type V = Map<nat, V::V>;
}
impl<V: View> Slab<V> {
    fn set_slot(&mut self, rel: u64, value: V) {
        let idx = rel as usize;
        self.inner.set(idx, Some(value));
    }
}
"#,
    );
    assert!(cpp.contains("this->inner[idx] = rusty::Option<V>(std::move(value));"), "{cpp}");
    assert!(!cpp.contains(".set("), "{cpp}");
    assert!(!cpp.contains("Rust-only dependent associated type alias"), "{cpp}");
    assert!(!cpp.contains("vstd"), "{cpp}");
}
