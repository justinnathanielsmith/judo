pub struct FooterItem {
    pub key: &'static str,
    pub desc: &'static str,
    pub highlighted: bool,
}

pub struct FooterGroup {
    pub name: &'static str,
    pub items: Vec<FooterItem>,
}
