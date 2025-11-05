use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::hint::black_box;
use std::time::Instant;

/// Tunables: bump these to turn it to 11
const N_PRED: usize = 32; // number of base predicates (<=64 for u64)
const N_RULES: usize = 64; // number of rules
const TERMS_PER_RULE: std::ops::RangeInclusive<usize> = 2..=4;
const REQ_PER_TERM: std::ops::RangeInclusive<usize> = 3..=6;
const FORB_PER_TERM: std::ops::RangeInclusive<usize> = 0..=2;
const N_EVENTS: usize = 100_000; // number of events to evaluate

// A rule is an OR of terms; each term is (all req) AND (none forb)
#[derive(Clone)]
struct Term {
    req_mask: u64,
    forb_mask: u64,
    req_idx: Vec<u8>,
    forb_idx: Vec<u8>,
}
#[derive(Clone)]
struct Rule {
    terms: Vec<Term>,
}

fn main() {
    assert!(
        N_PRED <= 64,
        "This demo uses a single u64; raise to u128 or bitvec for more."
    );

    //create reproducible rule set (same for both evaluators)
    let mut rng = SmallRng::seed_from_u64(0xB17B17);
    let rules = gen_rules(&mut rng);

    //create a population of events as base predicate booleans
    //    (think: these are the atomic facts your system derives per event)
    let preds: Vec<[bool; N_PRED]> = (0..N_EVENTS)
        .map(|_| random_predicate_row(&mut rng))
        .collect();

    //create bitsets (to_bits) once for the mask path, we time this to be fair....
    let t_to_bits_start = Instant::now();
    let bitsets: Vec<u64> = preds.iter().map(|row| to_bits(row)).collect();
    let t_to_bits = t_to_bits_start.elapsed();

    //but, both evaluators must agree on a few samples
    for i in 0..5 {
        let naive = eval_rules_naive(&rules, &preds[i]);
        let mask = eval_rules_mask(&rules, bitsets[i]);
        println!(
            "sample {i}: naive={naive} mask={mask} equal? {}",
            naive == mask
        );
    }
    println!();

    //naive boolean evaluation
    let t_naive_start = Instant::now();
    let mut count_naive = 0usize;
    for row in &preds {
        if black_box(eval_rules_naive(&rules, row)) {
            count_naive += 1;
        }
    }
    let t_naive = t_naive_start.elapsed();

    //masked evaluation (using precomputed bitsets)
    let t_mask_start = Instant::now();
    let mut count_mask = 0usize;
    for &state in &bitsets {
        if black_box(eval_rules_mask(&rules, state)) {
            count_mask += 1;
        }
    }
    let t_mask = t_mask_start.elapsed();

    //equivalence confirmation and print timings
    println!(
        "speedup (mask vs naive): {:.1}×",
        t_naive.as_secs_f64() / t_mask.as_secs_f64()
    );
    println!(
        "amortized cost per event: to_bits={:.3} µs, mask_eval={:.3} µs, naive={:.3} µs",
        1e6 * t_to_bits.as_secs_f64() / N_EVENTS as f64,
        1e6 * t_mask.as_secs_f64() / N_EVENTS as f64,
        1e6 * t_naive.as_secs_f64() / N_EVENTS as f64
    );
    println!(
        "events matched: naive={count_naive}  mask={count_mask}  equal? {}",
        count_naive == count_mask
    );
    println!("timings over {N_EVENTS} events, {N_RULES} rules, {N_PRED} predicates:");
    println!("  to_bits (prep once) : {:?}", t_to_bits);
    println!("  naive eval (booleans): {:?}", t_naive);
    println!("  mask  eval (bitwise) : {:?}", t_mask);
    println!(
        "\nTip: run with `--release`, then try N_RULES=256 or N_PRED=48 (switch to u128) for bigger gaps."
    );
}

/// the rulez

fn gen_rules(rng: &mut SmallRng) -> Vec<Rule> {
    (0..N_RULES).map(|_| gen_rule(rng)).collect()
}

fn gen_rule(rng: &mut SmallRng) -> Rule {
    let n_terms = rng.random_range(TERMS_PER_RULE);
    let mut terms = Vec::with_capacity(n_terms);
    for _ in 0..n_terms {
        let k_req = rng.random_range(REQ_PER_TERM);
        let k_forb = rng.random_range(FORB_PER_TERM);

        // sample distinct predicate indices for req/forb
        let req_idx = sample_distinct(rng, k_req);
        let forb_idx = sample_distinct_excluding(rng, k_forb, &req_idx);

        let mut req_mask = 0u64;
        let mut forb_mask = 0u64;
        for &i in &req_idx {
            req_mask |= 1u64 << i;
        }
        for &i in &forb_idx {
            forb_mask |= 1u64 << i;
        }

        terms.push(Term {
            req_mask,
            forb_mask,
            req_idx,
            forb_idx,
        });
    }
    Rule { terms }
}

fn sample_distinct(rng: &mut SmallRng, k: usize) -> Vec<u8> {
    use rand::seq::index::sample;
    if k == 0 {
        return vec![];
    }
    let picks = sample(rng, N_PRED, k);
    picks.into_iter().map(|i| i as u8).collect()
}

fn sample_distinct_excluding(rng: &mut SmallRng, k: usize, exclude: &[u8]) -> Vec<u8> {
    if k == 0 {
        return vec![];
    }
    let mut pool: Vec<u8> = (0..N_PRED as u8).collect();
    pool.retain(|i| !exclude.contains(i));
    // simple fisher-yates style picks
    let mut out = Vec::with_capacity(k);
    for _ in 0..k {
        if pool.is_empty() {
            break;
        }
        let idx = rng.random_range(0..pool.len());
        out.push(pool.swap_remove(idx));
    }
    out
}

/// event predicate rows

fn random_predicate_row(rng: &mut SmallRng) -> [bool; N_PRED] {
    // bias, some preds to true more often to create realistic “hits”.
    // early bits = rarer. later bits = more common (tunable).
    let mut row = [false; N_PRED];
    for i in 0..N_PRED {
        let p_true = 0.10 + (i as f64 / N_PRED as f64) * 0.35; // ~10%..45%
        row[i] = rng.random_bool(p_true);
    }
    row
}

/// to_bits()
#[inline]
fn to_bits(row: &[bool; N_PRED]) -> u64 {
    let mut s = 0u64;
    // NOTE: this loop is intentionally explicit (no iter::enumerate)
    for i in 0..N_PRED {
        if row[i] {
            s |= 1u64 << i;
        }
    }
    s
}

/// evaluators
/// naive... check each term by walking the boolean vector.
#[inline]
fn eval_rules_naive(rules: &Vec<Rule>, row: &[bool; N_PRED]) -> bool {
    'rule: for r in rules {
        for t in &r.terms {
            // (all must be true)
            for &i in &t.req_idx {
                if !row[i as usize] {
                    continue /* to next term */;
                }
            }
            // (no forbidden true)
            let mut any_forb = false;
            for &i in &t.forb_idx {
                if row[i as usize] {
                    any_forb = true;
                    break;
                }
            }
            if !any_forb {
                // term matched and rule satisfied
                continue 'rule;
            }
        }
        // no term matched for this rule
        return false;
    }
    true
}

/// masked: 2 ANDs + 2 compares-per -term "branch-predictable"
#[inline]
fn eval_rules_mask(rules: &Vec<Rule>, state: u64) -> bool {
    'rule: for r in rules {
        for t in &r.terms {
            if (state & t.req_mask) == t.req_mask && (state & t.forb_mask) == 0 {
                continue 'rule;
            }
        }
        return false;
    }
    true
}
