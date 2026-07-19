use anyhow::Context;
use anyhow::Result;
use codex_model_provider_info::OLLAMA_OSS_PROVIDER_ID;
use codex_model_provider_info::built_in_model_providers;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use pretty_assertions::assert_eq;
use serde_json::Value;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ollama_combines_base_instructions_with_initial_developer_context() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let response_mock = responses::mount_sse_once(
        &server,
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_completed("resp-1"),
        ]),
    )
    .await;
    let base_url = format!("{}/v1", server.uri());
    let mut builder = test_codex().with_config(move |config| {
        let mut provider =
            built_in_model_providers(/*openai_base_url*/ None)[OLLAMA_OSS_PROVIDER_ID].clone();
        provider.base_url = Some(base_url);
        config.model_provider_id = OLLAMA_OSS_PROVIDER_ID.to_string();
        config.model_provider = provider;
        config.base_instructions = Some("Base instructions for Ollama.".to_string());
        config.developer_instructions = Some("Existing developer context.".to_string());
        config.include_permissions_instructions = false;
        config.include_apps_instructions = false;
        config.include_collaboration_mode_instructions = false;
        config.include_skill_instructions = false;
        config.include_environment_context = false;
    });
    let test = builder.build_with_auto_env(&server).await?;

    test.submit_turn("hello").await?;

    let request = response_mock.single_request();
    let body = request.body_json();
    assert!(body.get("instructions").is_none());
    let input = body["input"]
        .as_array()
        .context("Responses request input should be an array")?;
    assert_eq!(
        input
            .iter()
            .filter(|item| item.get("role").and_then(Value::as_str) == Some("developer"))
            .count(),
        1
    );
    assert_eq!(input[0]["role"], "developer");
    assert_eq!(
        request.message_input_text_groups("developer"),
        vec![vec![
            "Base instructions for Ollama.".to_string(),
            "Existing developer context.".to_string(),
        ]]
    );
    assert_eq!(request.message_input_texts("user"), vec!["hello"]);

    Ok(())
}
