use warpui::AppContext;

/// Local replacement for the removed client experiment framework.
pub enum BlockOnboarding {
    VariantOne,
    VariantTwo,
}

impl BlockOnboarding {
    pub fn get_group(_ctx: &mut AppContext) -> Option<Self> {
        None
    }
}

/// Improved palette search is a local feature in Warper, not a rollout experiment.
pub struct ImprovedPaletteSearch;

impl ImprovedPaletteSearch {
    pub fn improved_search_enabled(_ctx: &mut AppContext) -> bool {
        true
    }
}

pub fn init(_ctx: &mut AppContext) {}
