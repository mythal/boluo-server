macro_rules! regex {
    ($pattern: expr) => {{
        use once_cell::sync::OnceCell;
        use regex::Regex;
        static CELL: OnceCell<Regex> = OnceCell::new();
        CELL.get_or_init(|| Regex::new($pattern).unwrap())
    }};
}
