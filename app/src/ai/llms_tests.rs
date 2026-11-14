use super::*;

#[test]
fn llm_info_deserializes_without_base_model_name() {
    let raw = r#"{
            "display_name": "gpt-4o",
            "id": "gpt-4o",
            "usage_metadata": {
                "request_multiplier": 1,
                "credit_multiplier": null
            },
            "description": null,
            "disable_reason": null,
            "vision_supported": false,
            "spec": null,
            "provider": "Unknown"
        }"#;

    let info: LLMInfo = serde_json::from_str(raw).expect("should deserialize");
    assert_eq!(info.display_name, "gpt-4o");
    assert_eq!(info.base_model_name, "gpt-4o");
}

#[test]
fn llm_info_deserializes_host_configs_as_vec() {
    // Wire format from server: host_configs is a Vec
    let raw = r#"{
            "display_name": "gpt-4o",
            "id": "gpt-4o",
            "usage_metadata": { "request_multiplier": 1, "credit_multiplier": null },
            "provider": "OpenAI",
            "host_configs": [
                { "enabled": true, "model_routing_host": "DirectApi" },
                { "enabled": false, "model_routing_host": "AwsBedrock" }
            ]
        }"#;

    let info: LLMInfo = serde_json::from_str(raw).expect("should deserialize vec format");
    assert_eq!(info.display_name, "gpt-4o");
    assert_eq!(info.host_configs.len(), 2);
    assert!(
        info.host_configs
            .get(&LLMModelHost::DirectApi)
            .unwrap()
            .enabled
    );
    assert!(
        !info
            .host_configs
            .get(&LLMModelHost::AwsBedrock)
            .unwrap()
            .enabled
    );
}

#[test]
fn llm_info_round_trip_serializes_and_deserializes() {
    // Start with wire format (Vec)
    let wire_json = r#"{
            "display_name": "claude-3",
            "base_model_name": "claude-3",
            "id": "claude-3",
            "usage_metadata": { "request_multiplier": 2, "credit_multiplier": 1.5 },
            "description": "A powerful model",
            "vision_supported": true,
            "provider": "Anthropic",
            "host_configs": [
                { "enabled": true, "model_routing_host": "DirectApi" }
            ]
        }"#;

    // Deserialize from wire format
    let info: LLMInfo = serde_json::from_str(wire_json).expect("should deserialize");

    // Serialize (produces HashMap format)
    let serialized = serde_json::to_string(&info).expect("should serialize");

    // Deserialize again (from HashMap format)
    let round_tripped: LLMInfo =
        serde_json::from_str(&serialized).expect("should deserialize after round trip");

    assert_eq!(info, round_tripped);
}

#[test]
fn openrouter_custom_model_query_accepts_model_ids() {
    assert!(is_openrouter_custom_model_query(
        "anthropic/claude-sonnet-4"
    ));
    assert!(is_openrouter_custom_model_query(
        "google/gemini-2.5-pro:thinking"
    ));
}

#[test]
fn openrouter_custom_model_query_rejects_empty_auto_and_text() {
    assert!(!is_openrouter_custom_model_query(""));
    assert!(!is_openrouter_custom_model_query(
        DEFAULT_OPENROUTER_MODEL_ID
    ));
    assert!(!is_openrouter_custom_model_query("claude sonnet"));
}

#[test]
fn openrouter_custom_model_is_added_to_all_model_groups() {
    let mut models = openrouter_models_by_feature();
    let model_id = "deepseek/deepseek-chat-v3.1";

    assert!(models.ensure_openrouter_custom_model(Some(model_id)));
    let llm_id = LLMId::from(model_id);

    assert_eq!(
        models
            .agent_mode
            .info_for_id(&llm_id)
            .map(|llm| &llm.provider),
        Some(&LLMProvider::OpenRouter)
    );
    assert!(models.coding.info_for_id(&llm_id).is_some());
    assert!(models
        .cli_agent
        .as_ref()
        .is_some_and(|choices| choices.info_for_id(&llm_id).is_some()));
    assert!(models
        .computer_use
        .as_ref()
        .is_some_and(|choices| choices.info_for_id(&llm_id).is_some()));
}

#[test]
fn openrouter_model_metadata_maps_to_llm_info() {
    let raw = r#"{
        "id": "openai/gpt-4.1",
        "name": "OpenAI: GPT-4.1",
        "description": "Flagship OpenAI model",
        "architecture": {
            "input_modalities": ["text", "image"]
        }
    }"#;

    let model: OpenRouterModel = serde_json::from_str(raw).expect("should deserialize");
    let llm = model.into_llm().expect("should convert");

    assert_eq!(llm.id.as_str(), "openai/gpt-4.1");
    assert_eq!(llm.display_name, "OpenAI: GPT-4.1");
    assert_eq!(llm.provider, LLMProvider::OpenRouter);
    assert!(llm.vision_supported);
}
