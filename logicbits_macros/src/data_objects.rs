#[derive(Debug)]
pub enum FieldAttr {
    Diner { pred: String },
    Kitchen { eq: String, pred: String },
    Largeness { pred_prefix: String, heat: Vec<u32> },
    Serve { pred_ns: String },
}
