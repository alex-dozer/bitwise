use logicbits::{KitchenNightmares, ToBits};
use serde::Deserialize;

#[derive(KitchenNightmares)]
#[yuck(kitchen_menu = 1)]
pub struct Event<'a> {
    #[yuck(diner(eq = "acme", pred = "DINER_ACME"))]
    diner: &'a str,

    #[yuck(kitchen(pred = "BIG_GROUP"))]
    big_group: bool,

    #[yuck(serve(pred_ns = "NO_SERVE"))]
    serve: u16,

    #[yuck(largeness(pred_prefix = "LRGNSS_ORDER_", heat = "100,200,600"))]
    heat: u32,
}

#[derive(Deserialize)]
struct PolicySpec {
    name: String,
    all: Vec<String>,
    none: Vec<String>,
}

fn main() {
    let e = Event {
        diner: "acme",
        big_group: false,
        serve: 200,
        heat: 230,
    };

    let bits = e.to_bits();
    println!("bits={bits:#b} kitchen_menu={}", Event::KITCHEN_MENU);

    // sanity: make sure names are registered
    println!(
        "known? DINER_ACME={}, LRGNSS_ORDER_200={}, BIG_GROUP={}",
        Event::pred_mask_by_name("DINER_ACME").is_some(),
        Event::pred_mask_by_name("LRGNSS_ORDER_200").is_some(),
        Event::pred_mask_by_name("BIG_GROUP").is_some(),
    );

    let spec = PolicySpec {
        name: "heat_warn".into(),
        all: vec!["DINER_ACME".into(), "LRGNSS_ORDER_200".into()],
        none: vec!["BIG_GROUP".into()],
    };

    let mut req = 0u64;
    let mut forb = 0u64;
    for n in spec.all {
        req |= Event::pred_mask_by_name(&n).expect("unknown pred");
    }
    for n in spec.none {
        forb |= Event::pred_mask_by_name(&n).expect("unknown pred");
    }

    let matched = (bits & req) == req && (bits & forb) == 0;
    println!("matched '{}'? {}", spec.name, matched);
}
