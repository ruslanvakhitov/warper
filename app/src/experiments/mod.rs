use warpui::AppContext;

/// Improved palette search is a local feature in Warper, not a rollout experiment.
pub struct ImprovedPaletteSearch;

impl ImprovedPaletteSearch {
    pub fn improved_search_enabled(_ctx: &mut AppContext) -> bool {
        true
    }
}

#[cfg(test)]
pub fn init(_ctx: &mut AppContext) {}
