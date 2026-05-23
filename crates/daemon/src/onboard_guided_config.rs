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

pub(super) fn resolve_model_selection(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
    guided_prompt_path: GuidedPromptPath,
    available_models: &[String],
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<String> {
    let prompt_default = onboarding_model_policy::resolve_onboarding_model_prompt_default(
        &config.provider,
        options.model.as_deref(),
    )?;

    if options.non_interactive {
        return Ok(prompt_default);
    }

    print_lines(
        ui,
        render_model_selection_screen_lines_with_style(
            config,
            prompt_default.as_str(),
            guided_prompt_path,
            context.render_width,
            true,
            !available_models.is_empty(),
        ),
    )?;
    if !available_models.is_empty() {
        // When we render the model catalog choices from a static provider list,
        // we still compute `prompt_default` (often `auto`) for the prompt UI.
        // Hide `auto` from the selectable catalog to match operator expectations.
        let hide_prompt_default_from_catalog = prompt_default.trim().eq_ignore_ascii_case("auto")
            && is_volcengine_coding_plan_domestic_static_catalog(&config.provider);

        let effective_prompt_default = if hide_prompt_default_from_catalog {
            ""
        } else {
            prompt_default.as_str()
        };

        let catalog_choices = onboarding_model_policy::onboarding_model_catalog_choices(
            effective_prompt_default,
            available_models,
        );
        let (select_options, default_idx) = build_model_selection_options(&catalog_choices);
        let idx = ui.select_one(
            "Model",
            &select_options,
            default_idx,
            SelectInteractionMode::Search,
        )?;
        let selected = select_options
            .get(idx)
            .ok_or_else(|| format!("model selection index {idx} out of range"))?;
        if selected.slug != ONBOARD_CUSTOM_MODEL_OPTION_SLUG {
            return Ok(selected.slug.clone());
        }
        let custom_model = ui.prompt_with_default("Custom model id", effective_prompt_default)?;
        let trimmed = custom_model.trim();
        if trimmed.is_empty() {
            return Err("model cannot be empty".to_owned());
        }
        return Ok(trimmed.to_owned());
    }
    let value = ui.prompt_with_default("Model", prompt_default.as_str())?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("model cannot be empty".to_owned());
    }
    Ok(trimmed.to_owned())
}

pub(super) async fn load_onboarding_model_catalog(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
) -> Vec<String> {
    // Volcano Engine "Coding Plan" domestic endpoint has a stable, operator-provided model list.
    // Using it avoids an interactive onboarding dependency on `GET /models`.
    if is_volcengine_coding_plan_domestic_static_catalog(&config.provider) {
        return vec![
            // Keep the historical default model id as an explicit choice.
            "ark-code-latest".to_owned(),
            "doubao-seed-2.0-code".to_owned(),
            "doubao-seed-2.0-pro".to_owned(),
            "doubao-seed-2.0-lite".to_owned(),
            "doubao-seed-code".to_owned(),
            "minimax-m2.5".to_owned(),
            "glm-4.7".to_owned(),
            "deepseek-v3.2".to_owned(),
            "kimi-k2.5".to_owned(),
        ];
    }

    if options.non_interactive || options.skip_model_probe {
        return Vec::new();
    }
    let has_provider_credentials = mvp::provider::provider_auth_ready(config).await;
    let provider_requires_explicit_auth = config.provider.requires_explicit_auth_configuration();
    if !has_provider_credentials && provider_requires_explicit_auth {
        return Vec::new();
    }
    mvp::provider::fetch_available_models(config)
        .await
        .unwrap_or_default()
}

pub(super) fn is_volcengine_coding_plan_domestic_static_catalog(
    provider: &mvp::config::ProviderConfig,
) -> bool {
    if provider.kind != mvp::config::ProviderKind::VolcengineCoding {
        return false;
    }

    let Ok(actual_url) = reqwest::Url::parse(provider.resolved_base_url().trim()) else {
        return false;
    };
    let Ok(canonical_url) = reqwest::Url::parse(
        mvp::config::ProviderKind::VolcengineCoding
            .profile()
            .base_url,
    ) else {
        return false;
    };

    actual_url.scheme() == canonical_url.scheme()
        && actual_url.host_str() == canonical_url.host_str()
        && actual_url.port_or_known_default() == canonical_url.port_or_known_default()
        && actual_url.path().trim_end_matches('/') == canonical_url.path().trim_end_matches('/')
}

pub(super) fn build_model_selection_options(
    catalog_choices: &onboarding_model_policy::OnboardingModelCatalogChoices,
) -> (Vec<SelectOption>, Option<usize>) {
    let default_idx = catalog_choices.default_index;
    let mut options = Vec::new();

    for (index, model) in catalog_choices.ordered_models.iter().enumerate() {
        let is_default_model = default_idx == Some(index);
        let description = if is_default_model {
            "current or suggested default".to_owned()
        } else {
            String::new()
        };

        let option = SelectOption {
            label: model.clone(),
            slug: model.clone(),
            description,
            recommended: is_default_model,
        };
        options.push(option);
    }

    options.push(SelectOption {
        label: "enter custom model id".to_owned(),
        slug: ONBOARD_CUSTOM_MODEL_OPTION_SLUG.to_owned(),
        description: "manually type any provider model id".to_owned(),
        recommended: false,
    });

    (options, default_idx)
}

pub(super) fn resolve_api_key_env_selection(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
    default_api_key_env: String,
    guided_prompt_path: GuidedPromptPath,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<String> {
    let explicit_selection = if let Some(api_key_env) = options.api_key_env.as_deref() {
        if is_explicit_onboard_clear_input(api_key_env) {
            return Ok(String::new());
        }
        let trimmed = api_key_env.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(validate_selected_provider_credential_env(config, trimmed)?)
        }
    } else {
        None
    };

    if options.non_interactive {
        return Ok(explicit_selection.unwrap_or(default_api_key_env));
    }
    let initial = explicit_selection
        .as_deref()
        .unwrap_or(default_api_key_env.as_str());
    let example_env_name =
        provider_credential_policy::provider_credential_env_hint(&config.provider)
            .unwrap_or_else(|| "PROVIDER_API_KEY".to_owned());
    loop {
        print_lines(
            ui,
            render_api_key_env_selection_screen_lines_with_style(
                config,
                default_api_key_env.as_str(),
                initial,
                guided_prompt_path,
                context.render_width,
                true,
            ),
        )?;
        let value = ui.prompt_with_default("Credential env var name", initial)?;
        if is_explicit_onboard_clear_input(&value) {
            return Ok(String::new());
        }
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }
        match validate_selected_provider_credential_env(config, trimmed) {
            Ok(validated) => return Ok(validated),
            Err(error) => {
                print_message(ui, error)?;
                print_message(
                    ui,
                    format!(
                        "enter the environment variable name only, for example {example_env_name}, or type :clear to remove the env binding"
                    ),
                )?;
            }
        }
    }
}

pub(super) fn apply_selected_api_key_env(
    provider: &mut mvp::config::ProviderConfig,
    selected_api_key_env: String,
) {
    let selected_api_key_env = selected_api_key_env.trim();
    if selected_api_key_env.is_empty() {
        provider.clear_api_key_env_binding();
        provider.clear_oauth_access_token_env_binding();
        return;
    }

    provider.api_key = None;
    provider.oauth_access_token = None;
    match provider_credential_policy::selected_provider_credential_env_field(
        provider,
        selected_api_key_env,
    ) {
        provider_credential_policy::ProviderCredentialEnvField::ApiKey => {
            provider.clear_oauth_access_token_env_binding();
            provider.set_api_key_env_binding(Some(selected_api_key_env.to_owned()));
        }
        provider_credential_policy::ProviderCredentialEnvField::OAuthAccessToken => {
            provider.clear_api_key_env_binding();
            provider.set_oauth_access_token_env_binding(Some(selected_api_key_env.to_owned()));
        }
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

pub(super) async fn resolve_web_search_provider_selection(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
    guided_prompt_path: GuidedPromptPath,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<String> {
    let explicit_override = explicit_web_search_provider_override(options)?;
    if mvp::provider::native_query_search_active(config) && explicit_override.is_none() {
        return Ok(current_web_search_provider(config).to_owned());
    }

    let recommendation = resolve_web_search_provider_recommendation(options, config).await?;
    let recommended_provider = recommendation.provider;
    let default_provider =
        resolve_effective_web_search_default_provider(options, config, &recommendation);

    if options.non_interactive {
        return Ok(default_provider.to_owned());
    }

    let screen_options = build_web_search_provider_screen_options(config, recommended_provider);
    let select_options = select_options_from_screen_options(&screen_options);
    let default_idx = screen_options
        .iter()
        .position(|option| option.key == default_provider);

    print_lines(
        ui,
        render_web_search_provider_selection_screen_lines_with_style(
            config,
            recommended_provider,
            default_provider,
            recommendation.reason.as_str(),
            guided_prompt_path,
            context.render_width,
            true,
        ),
    )?;
    let idx = ui.select_one(
        crate::access_terms::QUERY_SEARCH_PROVIDER_LABEL,
        &select_options,
        default_idx,
        SelectInteractionMode::List,
    )?;
    let selected = select_options
        .get(idx)
        .ok_or_else(|| crate::access_terms::query_search_provider_selection_index_error(idx))?;
    Ok(selected.slug.clone())
}

pub(super) fn resolve_web_search_credential_selection(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
    provider: &str,
    guided_prompt_path: GuidedPromptPath,
    non_interactive: bool,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<WebSearchCredentialSelection> {
    let explicit_override = explicit_web_search_provider_override(options)?;
    if mvp::provider::native_query_search_active(config) && explicit_override.is_none() {
        return Ok(WebSearchCredentialSelection::KeepCurrent);
    }

    let Some(descriptor) = mvp::config::web_search_provider_descriptor(provider) else {
        return Ok(WebSearchCredentialSelection::KeepCurrent);
    };
    if !descriptor.requires_api_key {
        return Ok(WebSearchCredentialSelection::KeepCurrent);
    }

    let explicit_selection = if let Some(raw_env_name) = options.web_search_api_key_env.as_deref() {
        if is_explicit_onboard_clear_input(raw_env_name) {
            return Ok(WebSearchCredentialSelection::ClearConfigured);
        }

        let trimmed_env_name = raw_env_name.trim();
        if trimmed_env_name.is_empty() {
            None
        } else {
            let validated_env_name =
                validate_selected_web_search_credential_env(provider, trimmed_env_name)?;
            Some(validated_env_name)
        }
    } else {
        None
    };

    let prompt_default = preferred_query_search_credential_env_default(config, provider);
    if non_interactive {
        if let Some(explicit_env_name) = explicit_selection {
            return Ok(WebSearchCredentialSelection::UseEnv(explicit_env_name));
        }

        return Ok(if prompt_default.trim().is_empty() {
            WebSearchCredentialSelection::KeepCurrent
        } else {
            WebSearchCredentialSelection::UseEnv(prompt_default)
        });
    }

    let initial_value = explicit_selection
        .as_deref()
        .unwrap_or(prompt_default.as_str());
    let example_env_name = descriptor
        .default_api_key_env
        .or_else(|| descriptor.api_key_env_names.first().copied())
        .unwrap_or("WEB_SEARCH_API_KEY")
        .to_owned();
    loop {
        print_lines(
            ui,
            render_web_search_credential_selection_screen_lines_with_style(
                config,
                provider,
                initial_value,
                guided_prompt_path,
                context.render_width,
                true,
            ),
        )?;
        let value = ui.prompt_with_default(
            crate::access_terms::query_search_credential_prompt_label(),
            initial_value,
        )?;
        if is_explicit_onboard_clear_input(&value) {
            return Ok(WebSearchCredentialSelection::ClearConfigured);
        }
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(WebSearchCredentialSelection::KeepCurrent);
        }
        match validate_selected_web_search_credential_env(provider, trimmed) {
            Ok(validated) => return Ok(WebSearchCredentialSelection::UseEnv(validated)),
            Err(error) => {
                print_message(ui, error)?;
                print_message(
                    ui,
                    crate::access_terms::query_search_credential_input_hint(
                        example_env_name.as_str(),
                    ),
                )?;
            }
        }
    }
}

pub(super) fn build_web_search_provider_screen_options(
    config: &mvp::config::LoongConfig,
    recommended_provider: &str,
) -> Vec<OnboardScreenOption> {
    mvp::config::web_search_provider_descriptors()
        .iter()
        .map(|descriptor| {
            let mut detail_lines = vec![descriptor.description.to_owned()];
            if let Some(credential) = summarize_query_search_credential(config, descriptor.id) {
                detail_lines.push(format!("{}: {}", credential.label, credential.value));
            }
            OnboardScreenOption {
                key: descriptor.id.to_owned(),
                label: descriptor.display_name.to_owned(),
                detail_lines,
                recommended: descriptor.id == recommended_provider,
            }
        })
        .collect()
}

pub(super) fn render_web_search_provider_selection_screen_lines_with_style(
    config: &mvp::config::LoongConfig,
    recommended_provider: &str,
    default_provider: &str,
    recommendation_reason: &str,
    guided_prompt_path: GuidedPromptPath,
    width: usize,
    color_enabled: bool,
) -> Vec<String> {
    let current_provider = current_web_search_provider(config);
    let current_provider_label = query_search_provider_display_name(current_provider);
    let recommended_provider_label = query_search_provider_display_name(recommended_provider);
    let default_provider_label = query_search_provider_display_name(default_provider);
    let options = build_web_search_provider_screen_options(config, recommended_provider);
    let default_footer_description = if default_provider == current_provider {
        format!("keep {current_provider_label}")
    } else {
        format!("use {default_provider_label}")
    };

    render_onboard_choice_screen(
        OnboardHeaderStyle::Compact,
        width,
        crate::access_terms::CHOOSE_QUERY_SEARCH_TITLE,
        crate::access_terms::CHOOSE_QUERY_SEARCH_PROVIDER_TITLE,
        Some((GuidedOnboardStep::WebSearchProvider, guided_prompt_path)),
        vec![
            format!("- current provider: {current_provider_label}"),
            format!("- recommended provider: {recommended_provider_label}"),
            format!("- why this is recommended: {recommendation_reason}"),
        ],
        options,
        vec![render_default_choice_footer_line(
            "Enter",
            default_footer_description.as_str(),
        )],
        true,
        color_enabled,
    )
}

pub(super) fn onboard_credential_env_name_is_safe(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut config = mvp::config::LoongConfig::default();
    config.provider.api_key = Some(SecretRef::Env {
        env: trimmed.to_owned(),
    });
    config.provider.api_key_env = None;

    config.validate().is_ok()
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
