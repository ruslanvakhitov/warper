use url::Url;

#[derive(PartialEq, Debug)]
pub enum WarpWebLink {
    Local,
}

pub fn get_item_data_from_warp_link(_url: &Url) -> Option<WarpWebLink> {
    None
}
