//! Onboarding-specific AI types and conversions.

use onboarding::slides::OnboardingModelInfo;
use warp_core::ui::icons::Icon;

use super::llms::LLMInfo;

impl From<&LLMInfo> for OnboardingModelInfo {
    fn from(llm: &LLMInfo) -> Self {
        Self {
            id: llm.id.clone(),
            title: llm.display_name.clone(),
            icon: llm.provider.icon().unwrap_or(Icon::AgentMode),
            requires_upgrade: false,
            is_default: false,
        }
    }
}
