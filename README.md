# bitwise

**A derive macro that compiles annotated struct fields into bitmask predicates** for branchless, constant-time rule evaluation. It implements a tiny trait (`ToBits`) and generates stable predicate masks so complex boolean logic can be evaluated as simple bitwise checks.

> TL;DR - Replace a thicket of `if` chains with:
>
> ```rust
> (state & req_mask) == req_mask && (state & forb_mask) == 0
> ```
>
> where `state` is a `u64` with one bit per predicate.

---

## Why use a macro?

You *could* compute masks at runtime. But the macro gives you:

* **Compile-time contracts.** Fields ↔ predicates ↔ bit positions can’t drift.
* **Zero runtime reflection.** No stringly-typed lookups to wire logic.
* **Branchless hot path.** Once an event becomes a bitset, each rule term is two ANDs + two compares.
* **Auditability.** Predicate names are generated deterministically; you can serialize masks and policies.

This is not syntactic sugar; it’s a tiny domain-specific compiler.

---

## How it works

* Each decorated field generates **one or more predicate bits** (e.g., a thresholded field yields multiple bits like `LRGNSS_ORDER100`, `LRGNSS_ORDER200`, …).
* The derive emits:

  * A hidden module with an enum of predicates (stable indices → stable masks).
  * `impl ToBits for YourType` to build the `u64` state.
  * `YourType::pred_mask_by_name("PRED_NAME") -> Option<u64>` for ergonomic policy compilation.
  * `YourType::SCHEMA_VERSION` to tag mask layouts across schema changes.

Bit math 101:

* A **predicate** is a bit in `state: u64`.
* A **term** (conjunction) is a `req_mask` plus a `forb_mask`.
* A **rule** (disjunction of terms) matches if **any** term matches.

Checking a term is:

```rust
(state & req_mask) == req_mask && (state & forb_mask) == 0
```

No branches. No guessing.

---

## Attribute grammar

Struct level:

```rust
#[yuck(kitchen_menu = <u16>)] // schema version tag
```

Field level:

```rust
#[yuck(diner(eq = "<string>", pred = "<PRED_NAME>"))]       // string equality → 1 bit
#[yuck(kitchen(pred = "<PRED_NAME>"))]                      // boolean flag → 1 bit
#[yuck(serve(pred_ns = "<PRED_NAME>"))]                     // integer class (you decide semantics)
#[yuck(largeness(pred_prefix = "<PREFIX>", heat = "n1,n2"))]// thresholds → bits PREFIXn1, PREFIXn2
```

> **Note on names:** Threshold variants are concatenated as `PREFIX<value>` (e.g. `LRGNSS_ORDER200`).
> If you prefer `PREFIX_<value>`, that’s a one-line tweak in the macro.

---

## Minimal example

```rust
use logicbits::{ToBits, KitchenNightmares};

#[derive(KitchenNightmares)]
#[yuck(kitchen_menu = 1)]
pub struct Event<'a> {
    #[yuck(diner(eq = "acme", pred = "DINER_ACME"))]
    diner: &'a str,

    #[yuck(kitchen(pred = "BIG_GROUP"))]
    big_group: bool,

    #[yuck(serve(pred_ns = "NO_SERVE"))]
    serve: u16, // your semantics; e.g., set bit if in {…}

    #[yuck(largeness(pred_prefix = "LRGNSS_ORDER", heat = "100,200,600"))]
    heat: u32,
}

fn main() {
    let e = Event { diner: "acme", big_group: true, serve: 0, heat: 230 };
    let state = e.to_bits(); // u64 with bits set
    let m_acme = Event::pred_mask_by_name("DINER_ACME").unwrap();
    assert_eq!((state & m_acme) == m_acme, true);
}
```

---

## Generated API (what you can rely on)

* `impl ToBits for Event { fn to_bits(&self) -> u64 }`
* `impl Event {
    pub const SCHEMA_VERSION: u16;
    pub fn pred_mask_by_name(name: &str) -> Option<u64>;
    // (optional) pub const PREDICATES: &'static [&'static str]; // if you enable it
  }`

> **Schema versioning:** bump `#[yuck(kitchen_menu = N)]` when bit layouts change; downstream policy stores can reject mismatched versions.

---


Run it:

```bash
cargo run -p runner
```

---

```if-test```
Just a demonstration on why bitmasking is good.
*(On typical hardware you’ll see the mask path trounce the naive path; your numbers will vary.)*

---

## Limitations / gotchas

* `u64` supports up to 64 predicates. Use `u128` or multiple limbs for more.
* Threshold predicate names are concatenated (`PREFIX200`). Keep your policy names in sync.
* For string fields marked with `diner(eq="...")`, we emit case-insensitive compares by default; tweak as needed.

---

## FAQ

**Q: Why not do it all at runtime?**
A: Compile-time derivation prevents schema drift and gives you stable masks you can serialize, audit, and test against. It also keeps the hot path branchless.

**Q: Can I see the generated code?**
A: Yes-`cargo expand` is your friend. It’s the best way to learn how the macro translates your annotations.

**Q: Can I export all predicate names?**
A: The macro can emit a `PREDICATES: &[&str]` slice; enable that feature in the crate or copy the small snippet.


