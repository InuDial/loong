use super::*;

struct GuidedOnboardConfigUpdate {
    provider: mvp::config::ProviderConfig,
    model: String,
    selected_api_key_env: Option<String>,
    prompt_pack_id: Option<String>,
    personality: Option<mvp::prompt::PromptPersonality>,
    selected_system_prompt: Option<String>,
    selected_web_search_provider: String,
    web_search_credential_selection: WebSearchCredentialSelection,
}

pub(super) async fn apply_guided_onboard_configuration(
    options: &OnboardCommandOptions,
    output_path: &Path,
    provider_selection: &crate::migration::ProviderSelectionPlan,
    config: &mut mvp::config::LoongConfig,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<()> {
    let guided_prompt_path = resolve_guided_prompt_path(options, config);
    let update = collect_guided_onboard_config_update(
        options,
        provider_selection,
        config,
        guided_prompt_path,
        ui,
        context,
    )
    .await?;

    config.provider = update.provider;
    config.provider.model = update.model;

    if config.provider.kind == mvp::config::ProviderKind::GithubCopilot {
        finalize_github_copilot_onboard_credentials(
            &mut config.provider,
            output_path,
            options.non_interactive,
        )
        .await?;
    } else if let Some(selected_api_key_env) = update.selected_api_key_env {
        apply_selected_api_key_env(&mut config.provider, selected_api_key_env);
    }

    apply_guided_prompt_configuration(
        config,
        update.prompt_pack_id,
        update.personality,
        update.selected_system_prompt,
    );

    if let Some(profile_raw) = options.memory_profile.as_deref() {
        config.memory.profile = parse_memory_profile(profile_raw).ok_or_else(|| {
            format!(
                "unsupported --memory-profile value \"{profile_raw}\". supported: {}",
                supported_memory_profile_list()
            )
        })?;
    }

    config.tools.web_search.default_provider = update.selected_web_search_provider.clone();
    apply_selected_web_search_credential(
        config,
        update.selected_web_search_provider.as_str(),
        update.web_search_credential_selection,
    )?;

    Ok(())
}

async fn collect_guided_onboard_config_update(
    options: &OnboardCommandOptions,
    provider_selection: &crate::migration::ProviderSelectionPlan,
    config: &mvp::config::LoongConfig,
    guided_prompt_path: GuidedPromptPath,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<GuidedOnboardConfigUpdate> {
    let provider = resolve_provider_selection(
        options,
        config,
        provider_selection,
        guided_prompt_path,
        ui,
        context,
    )?;
    let mut draft_config = config.clone();
    draft_config.provider = provider.clone();

    let available_models = load_onboarding_model_catalog(options, &draft_config).await;
    let model = resolve_model_selection(
        options,
        &draft_config,
        guided_prompt_path,
        &available_models,
        ui,
        context,
    )?;
    draft_config.provider.model = model.clone();

    let selected_api_key_env =
        if draft_config.provider.kind == mvp::config::ProviderKind::GithubCopilot {
            None
        } else {
            let default_api_key_env = preferred_api_key_env_default(&draft_config);
            Some(resolve_api_key_env_selection(
                options,
                &draft_config,
                default_api_key_env,
                guided_prompt_path,
                ui,
                context,
            )?)
        };

    let selected_system_prompt = resolve_guided_system_prompt_selection(
        options,
        &draft_config,
        guided_prompt_path,
        ui,
        context,
    )?;
    let prompt_pack_id = if guided_prompt_path == GuidedPromptPath::NativePromptPack {
        Some(mvp::prompt::DEFAULT_PROMPT_PACK_ID.to_owned())
    } else {
        None
    };
    let personality =
        if guided_prompt_path == GuidedPromptPath::NativePromptPack && options.non_interactive {
            options
                .personality
                .as_deref()
                .map(|personality_raw| {
                    parse_prompt_personality(personality_raw).ok_or_else(|| {
                        format!(
                            "unsupported --personality value \"{personality_raw}\". supported: {}",
                            supported_personality_list()
                        )
                    })
                })
                .transpose()?
        } else {
            draft_config.cli.personality
        };

    let selected_web_search_provider = resolve_web_search_provider_selection(
        options,
        &draft_config,
        guided_prompt_path,
        ui,
        context,
    )
    .await?;
    draft_config.tools.web_search.default_provider = selected_web_search_provider.clone();
    let web_search_credential_selection = resolve_web_search_credential_selection(
        options,
        &draft_config,
        selected_web_search_provider.as_str(),
        guided_prompt_path,
        options.non_interactive,
        ui,
        context,
    )?;

    Ok(GuidedOnboardConfigUpdate {
        provider,
        model,
        selected_api_key_env,
        prompt_pack_id,
        personality,
        selected_system_prompt,
        selected_web_search_provider,
        web_search_credential_selection,
    })
}

fn resolve_guided_system_prompt_selection(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
    guided_prompt_path: GuidedPromptPath,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<Option<String>> {
    match guided_prompt_path {
        GuidedPromptPath::NativePromptPack => Ok(None),
        GuidedPromptPath::InlineOverride => {
            if options.non_interactive {
                return Ok(options.system_prompt.clone().map(|system_prompt| {
                    if is_explicit_onboard_clear_input(system_prompt.as_str()) {
                        mvp::config::CliChannelConfig::default().system_prompt
                    } else {
                        system_prompt
                    }
                }));
            }

            let prompt_default = options
                .system_prompt
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if config.cli.uses_native_prompt_pack() {
                        String::new()
                    } else {
                        config.cli.system_prompt.clone()
                    }
                });
            print_lines(
                ui,
                render_system_prompt_selection_screen_lines_with_style(
                    config,
                    prompt_default.as_str(),
                    guided_prompt_path,
                    context.render_width,
                    true,
                ),
            )?;
            let value = ui.prompt_with_default("System prompt", prompt_default.as_str())?;
            if is_explicit_onboard_clear_input(&value) {
                return Ok(Some(mvp::config::CliChannelConfig::default().system_prompt));
            }
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed.to_owned()))
            }
        }
    }
}

fn apply_guided_prompt_configuration(
    config: &mut mvp::config::LoongConfig,
    prompt_pack_id: Option<String>,
    personality: Option<mvp::prompt::PromptPersonality>,
    selected_system_prompt: Option<String>,
) {
    config.cli.prompt_pack_id = prompt_pack_id;
    config.cli.personality = personality;
    apply_selected_system_prompt(config, selected_system_prompt);
}

pub(super) fn resolve_guided_prompt_path(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
) -> GuidedPromptPath {
    if options.system_prompt.is_some() {
        return GuidedPromptPath::InlineOverride;
    }
    if options
        .personality
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return GuidedPromptPath::NativePromptPack;
    }
    if options.non_interactive {
        if config.cli.uses_native_prompt_pack() {
            return GuidedPromptPath::NativePromptPack;
        }
        if !config.cli.system_prompt.trim().is_empty() {
            return GuidedPromptPath::InlineOverride;
        }
    }
    GuidedPromptPath::NativePromptPack
}

pub fn resolve_guided_prompt_path_label_for_test(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
) -> &'static str {
    match resolve_guided_prompt_path(options, config) {
        GuidedPromptPath::NativePromptPack => "native",
        GuidedPromptPath::InlineOverride => "inline",
    }
}

pub(super) fn apply_selected_system_prompt(
    config: &mut mvp::config::LoongConfig,
    system_prompt: Option<String>,
) {
    match system_prompt.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => {
            config.cli.prompt_pack_id = Some(String::new());
            config.cli.personality = None;
            config.cli.system_prompt_addendum = None;
            config.cli.system_prompt = value.to_owned();
        }
        _ => config.cli.refresh_native_system_prompt(),
    }
}

#[cfg(test)]
mod volcengine_coding_plan_catalog_tests {
    use super::*;

    #[test]
    fn volcengine_coding_plan_domestic_static_catalog_detects_cn_beijing_coding_v3() {
        let provider = mvp::config::ProviderConfig {
            kind: mvp::config::ProviderKind::VolcengineCoding,
            base_url: "https://ark.cn-beijing.volces.com/api/coding/v3".to_owned(),
            ..mvp::config::ProviderConfig::default()
        };

        assert!(is_volcengine_coding_plan_domestic_static_catalog(&provider));
    }

    #[test]
    fn volcengine_coding_plan_domestic_static_catalog_rejects_non_coding_plan_endpoints() {
        let provider = mvp::config::ProviderConfig {
            kind: mvp::config::ProviderKind::VolcengineCoding,
            base_url: "https://ark.cn-beijing.volces.com/api/v3".to_owned(),
            ..mvp::config::ProviderConfig::default()
        };

        assert!(!is_volcengine_coding_plan_domestic_static_catalog(
            &provider
        ));
    }

    #[test]
    fn volcengine_coding_plan_domestic_static_catalog_rejects_proxy_path() {
        let provider = mvp::config::ProviderConfig {
            kind: mvp::config::ProviderKind::VolcengineCoding,
            base_url: "https://proxy.example.com/api/coding/v3".to_owned(),
            ..mvp::config::ProviderConfig::default()
        };

        assert!(!is_volcengine_coding_plan_domestic_static_catalog(
            &provider
        ));
    }
}
