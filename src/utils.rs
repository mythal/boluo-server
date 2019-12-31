use uuid::Uuid;

macro_rules! regex {
    ($pattern: expr) => {{
        use once_cell::sync::OnceCell;
        use regex::Regex;
        static CELL: OnceCell<Regex> = OnceCell::new();
        CELL.get_or_init(|| Regex::new($pattern).unwrap())
    }};
}

pub fn id() -> Uuid {
    use once_cell::sync::OnceCell;
    use std::time::SystemTime;
    use uuid::v1::Context as UuidContext;
    use uuid::v1::Timestamp;

    static NODE_ID: OnceCell<[u8; 6]> = OnceCell::new();
    let node_id = NODE_ID.get_or_init(rand::random);
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!");
    static CONTEXT: UuidContext = UuidContext::new(0);
    let timestamp = Timestamp::from_unix(&CONTEXT, now.as_secs(), now.subsec_nanos());
    Uuid::new_v1(timestamp, node_id).expect("failed to generate UUID")
}
