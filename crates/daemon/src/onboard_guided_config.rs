use super::*;

pub(super) fn resolve_provider_selection(
    options: &OnboardCommandOptions,
    config: &mvp::config::LoongConfig,
    provider_selection: &crate::migration::ProviderSelectionPlan,
    guided_prompt_path: GuidedPromptPath,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<mvp::config::ProviderConfig> {
    if options.non_interactive {
        if let Some(provider_raw) = options.provider.as_deref() {
            return resolve_provider_config_from_selector(
                &config.provider,
                provider_selection,
                provider_raw,
            );
        }
        if provider_selection.requires_explicit_choice {
            let detected = provider_selection
                .imported_choices
                .iter()
                .map(|choice| choice.profile_id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "multiple detected provider choices found ({detected}); rerun with --provider {} to choose the active provider",
                crate::migration::provider_selection::PROVIDER_SELECTOR_PLACEHOLDER,
            ));
        }
        if let Some(default_profile_id) = provider_selection.default_profile_id.as_deref() {
            return resolve_provider_config_from_selector(
                &config.provider,
                provider_selection,
                default_profile_id,
            );
        }
        return Ok(crate::migration::resolve_provider_config_from_selection(
            &config.provider,
            provider_selection,
            provider_selection
                .default_kind
                .unwrap_or(config.provider.kind),
        ));
    }

    if !provider_selection.imported_choices.is_empty() {
        let select_options: Vec<SelectOption> = provider_selection
            .imported_choices
            .iter()
            .map(|choice| SelectOption {
                label: provider_kind_display_name(choice.kind).to_owned(),
                slug: choice.profile_id.clone(),
                description: format!("source: {}, summary: {}", choice.source, choice.summary),
                recommended: Some(choice.profile_id.as_str())
                    == provider_selection.default_profile_id.as_deref(),
            })
            .collect();
        let default_idx = if provider_selection.requires_explicit_choice {
            None
        } else {
            provider_selection
                .default_profile_id
                .as_deref()
                .and_then(|default_id| {
                    provider_selection
                        .imported_choices
                        .iter()
                        .position(|choice| choice.profile_id == default_id)
                })
        };
        print_lines(
            ui,
            render_provider_selection_header_lines(
                provider_selection,
                guided_prompt_path,
                context.render_width,
            ),
        )?;
        let idx = ui.select_one(
            "Provider",
            &select_options,
            default_idx,
            SelectInteractionMode::List,
        )?;
        let choice = provider_selection
            .imported_choices
            .get(idx)
            .ok_or_else(|| format!("provider selection index {idx} out of range"))?;
        return Ok(choice.config.clone());
    }

    // No imported choices — still use the numbered chooser so the provider
    // step stays aligned with the rest of onboarding.
    let default_provider_kind = options
        .provider
        .as_deref()
        .and_then(parse_provider_kind)
        .or(provider_selection.default_kind)
        .or_else(|| {
            provider_selection
                .default_profile_id
                .as_deref()
                .and_then(parse_provider_kind)
        })
        .unwrap_or(config.provider.kind);
    let provider_kinds = mvp::config::ProviderKind::all_sorted()
        .iter()
        .copied()
        .filter(|kind| {
            *kind != mvp::config::ProviderKind::Kimi
                && *kind != mvp::config::ProviderKind::KimiCoding
                && *kind != mvp::config::ProviderKind::Stepfun
                && *kind != mvp::config::ProviderKind::StepPlan
        })
        .collect::<Vec<_>>();
    let mut select_options: Vec<SelectOption> = provider_kinds
        .iter()
        .map(|kind| SelectOption {
            label: provider_kind_display_name(*kind).to_owned(),
            slug: provider_kind_id(*kind).to_owned(),
            description: String::new(),
            recommended: *kind == default_provider_kind,
        })
        .collect();
    select_options.push(SelectOption {
        label: "Kimi".to_owned(),
        slug: "kimi".to_owned(),
        description: "Kimi API or Kimi Coding".to_owned(),
        recommended: default_provider_kind == mvp::config::ProviderKind::Kimi
            || default_provider_kind == mvp::config::ProviderKind::KimiCoding,
    });
    select_options.push(SelectOption {
        label: "Stepfun".to_owned(),
        slug: "stepfun".to_owned(),
        description: "Stepfun API or Step Plan".to_owned(),
        recommended: default_provider_kind == mvp::config::ProviderKind::Stepfun
            || default_provider_kind == mvp::config::ProviderKind::StepPlan,
    });
    select_options.sort_by(|a, b| a.label.cmp(&b.label));
    let default_provider_slug = if matches!(
        default_provider_kind,
        mvp::config::ProviderKind::Kimi | mvp::config::ProviderKind::KimiCoding
    ) {
        "kimi"
    } else if matches!(
        default_provider_kind,
        mvp::config::ProviderKind::Stepfun | mvp::config::ProviderKind::StepPlan
    ) {
        "stepfun"
    } else {
        provider_kind_id(default_provider_kind)
    };
    let default_idx = if provider_selection.requires_explicit_choice {
        None
    } else {
        select_options
            .iter()
            .position(|option| option.slug == default_provider_slug)
    };
    print_lines(
        ui,
        render_provider_selection_header_lines(
            provider_selection,
            guided_prompt_path,
            context.render_width,
        ),
    )?;
    let idx = ui.select_one(
        "Provider",
        &select_options,
        default_idx,
        SelectInteractionMode::List,
    )?;
    let selected_slug = select_options
        .get(idx)
        .ok_or_else(|| format!("provider selection index {idx} out of range"))?
        .slug
        .clone();

    let kind: mvp::config::ProviderKind = if selected_slug == "kimi" {
        let kimi_options = vec![
            SelectOption {
                label: "Kimi API".to_owned(),
                slug: "kimi_api".to_owned(),
                description: "Standard Kimi chat completion API".to_owned(),
                recommended: true,
            },
            SelectOption {
                label: "Kimi Coding".to_owned(),
                slug: "kimi_coding".to_owned(),
                description: "Kimi for coding tasks".to_owned(),
                recommended: false,
            },
        ];
        print_lines(ui, vec!["Select the Kimi variant:".to_owned()])?;
        let kimi_default_idx = Some(usize::from(
            default_provider_kind == mvp::config::ProviderKind::KimiCoding,
        ));
        let sub_idx = ui.select_one(
            "Kimi variant",
            &kimi_options,
            kimi_default_idx,
            SelectInteractionMode::List,
        )?;
        let sub_slug = kimi_options
            .get(sub_idx)
            .ok_or_else(|| format!("kimi variant index {sub_idx} out of range"))?
            .slug
            .clone();
        if sub_slug == "kimi_coding" {
            mvp::config::ProviderKind::KimiCoding
        } else {
            mvp::config::ProviderKind::Kimi
        }
    } else if selected_slug == "stepfun" {
        let stepfun_options = vec![
            SelectOption {
                label: "Stepfun API".to_owned(),
                slug: "stepfun_api".to_owned(),
                description: "Standard Stepfun chat completion API".to_owned(),
                recommended: true,
            },
            SelectOption {
                label: "Step Plan".to_owned(),
                slug: "step_plan".to_owned(),
                description: "Step Plan for specialized tasks".to_owned(),
                recommended: false,
            },
        ];
        print_lines(ui, vec!["Select the Stepfun variant:".to_owned()])?;
        let stepfun_default_idx = Some(usize::from(
            default_provider_kind == mvp::config::ProviderKind::StepPlan,
        ));
        let sub_idx = ui.select_one(
            "Stepfun variant",
            &stepfun_options,
            stepfun_default_idx,
            SelectInteractionMode::List,
        )?;
        let sub_slug = stepfun_options
            .get(sub_idx)
            .ok_or_else(|| format!("stepfun variant index {sub_idx} out of range"))?
            .slug
            .clone();
        if sub_slug == "step_plan" {
            mvp::config::ProviderKind::StepPlan
        } else {
            mvp::config::ProviderKind::Stepfun
        }
    } else {
        provider_kinds
            .iter()
            .find(|kind| provider_kind_id(**kind) == selected_slug)
            .copied()
            .ok_or_else(|| format!("provider kind not found for slug {}", selected_slug))?
    };

    let mut provider_config =
        resolve_provider_config_from_selection(&config.provider, provider_selection, kind);

    if let Some(region_info) = kind.region_endpoint_info() {
        let configured_base_url = provider_config.base_url.as_str();
        let default_region_idx = region_info
            .variants
            .iter()
            .position(|variant| variant.base_url == configured_base_url)
            .unwrap_or(0);
        let region_options = region_info
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| {
                let is_default_variant = index == 0;
                let label = if is_default_variant {
                    format!("{} (default)", variant.label)
                } else {
                    variant.label.to_owned()
                };
                let slug = variant.base_url.to_owned();
                let description = format!("endpoint: {}", variant.base_url);
                let recommended = index == default_region_idx;
                SelectOption {
                    label,
                    slug,
                    description,
                    recommended,
                }
            })
            .collect::<Vec<_>>();
        let region_prompt = format!("Select the {} region endpoint:", region_info.family_label);
        print_lines(ui, vec![region_prompt])?;
        let region_idx = ui.select_one(
            "Region",
            &region_options,
            Some(default_region_idx),
            SelectInteractionMode::List,
        )?;
        let selected_base_url = region_options
            .get(region_idx)
            .ok_or_else(|| format!("region selection index {region_idx} out of range"))?
            .slug
            .clone();
        provider_config.set_base_url(selected_base_url);
    }

    prompt_provider_base_url_if_needed(options, kind, &mut provider_config, ui)?;

    Ok(provider_config)
}

pub(super) fn prompt_provider_base_url_if_needed(
    options: &OnboardCommandOptions,
    kind: mvp::config::ProviderKind,
    provider_config: &mut mvp::config::ProviderConfig,
    ui: &mut impl OnboardUi,
) -> CliResult<()> {
    let requires_custom_base_url = kind.requires_custom_base_url();
    if !requires_custom_base_url || options.non_interactive {
        return Ok(());
    }

    let configured_base_url = provider_config.base_url.trim().to_owned();
    let has_configured_base_url = !configured_base_url.is_empty();
    let has_unresolved_custom_base_url = provider_config.has_unresolved_custom_base_url();
    let prompt_lines = build_provider_base_url_prompt_lines(
        kind,
        configured_base_url.as_str(),
        has_unresolved_custom_base_url,
    );
    print_lines(ui, prompt_lines)?;

    let selected_base_url = if has_unresolved_custom_base_url || !has_configured_base_url {
        ui.prompt_required("Provider base URL")?
    } else {
        ui.prompt_with_default("Provider base URL", configured_base_url.as_str())?
    };
    let validated_base_url = validate_onboard_provider_base_url(selected_base_url.as_str())?;
    provider_config.set_base_url(validated_base_url);

    Ok(())
}

pub(super) fn build_provider_base_url_prompt_lines(
    kind: mvp::config::ProviderKind,
    configured_base_url: &str,
    has_unresolved_custom_base_url: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    let provider_label = provider_kind_display_name(kind);
    let intro_line = format!("Set the {} API base URL:", provider_label);
    lines.push(intro_line);

    if let Some(configuration_hint) = kind.configuration_hint() {
        lines.push(configuration_hint.to_owned());
    }

    if has_unresolved_custom_base_url {
        let template_line = format!("Current template: {}", configured_base_url.trim());
        lines.push(template_line);
    }

    lines
}

pub(super) fn validate_onboard_provider_base_url(raw: &str) -> CliResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("provider base URL cannot be empty".to_owned());
    }

    let parsed_url = reqwest::Url::parse(trimmed)
        .map_err(|error| format!("provider base URL is invalid: {error}"))?;
    let scheme = parsed_url.scheme();
    let valid_scheme = scheme == "http" || scheme == "https";
    if !valid_scheme {
        return Err("provider base URL must use http or https".to_owned());
    }

    let has_host = parsed_url.host_str().is_some();
    if !has_host {
        return Err("provider base URL must include a host".to_owned());
    }

    Ok(trimmed.to_owned())
}

pub fn resolve_provider_config_from_selector(
    current_provider: &mvp::config::ProviderConfig,
    provider_selection: &crate::migration::ProviderSelectionPlan,
    selector: &str,
) -> CliResult<mvp::config::ProviderConfig> {
    match crate::migration::resolve_choice_by_selector_resolution(provider_selection, selector) {
        crate::migration::ImportedChoiceSelectorResolution::Match(profile_id) => {
            let Some(choice) = provider_selection
                .imported_choices
                .iter()
                .find(|choice| choice.profile_id == profile_id)
            else {
                return Err(format!(
                    "provider selection plan is inconsistent: resolved profile `{profile_id}` is missing"
                ));
            };
            return Ok(choice.config.clone());
        }
        crate::migration::ImportedChoiceSelectorResolution::Ambiguous(profile_ids) => {
            return Err(crate::migration::format_ambiguous_selector_error(
                provider_selection,
                selector,
                &profile_ids,
            ));
        }
        crate::migration::ImportedChoiceSelectorResolution::NoMatch => {}
    }

    let kind = parse_provider_kind(selector).ok_or_else(|| {
        if provider_selection.imported_choices.is_empty() {
            return format!(
                "unsupported provider value \"{selector}\". accepted selectors: {}. {}",
                supported_provider_list(),
                crate::migration::provider_selection::PROVIDER_SELECTOR_NOTE,
            );
        }
        crate::migration::format_unknown_selector_error(
            provider_selection,
            format!("unsupported provider value \"{selector}\"").as_str(),
        )
    })?;
    let matching_choices = provider_selection
        .imported_choices
        .iter()
        .filter(|choice| choice.kind == kind)
        .collect::<Vec<_>>();
    if matching_choices.len() > 1 {
        let profile_ids = matching_choices
            .iter()
            .map(|choice| choice.profile_id.clone())
            .collect::<Vec<_>>();
        return Err(crate::migration::format_ambiguous_selector_error(
            provider_selection,
            selector,
            &profile_ids,
        ));
    }
    if let Some(choice) = matching_choices.first() {
        return Ok(choice.config.clone());
    }
    Ok(crate::migration::resolve_provider_config_from_selection(
        current_provider,
        provider_selection,
        kind,
    ))
}

pub fn build_provider_selection_plan_for_candidate(
    selected_candidate: &ImportCandidate,
    all_candidates: &[ImportCandidate],
) -> crate::migration::ProviderSelectionPlan {
    let migration_selected = migration_candidate_from_onboard(selected_candidate);
    let migration_candidates = all_candidates
        .iter()
        .map(migration_candidate_from_onboard)
        .collect::<Vec<_>>();
    crate::migration::build_provider_selection_plan_for_candidate(
        &migration_selected,
        &migration_candidates,
    )
}

pub fn resolve_provider_config_from_selection(
    current_provider: &mvp::config::ProviderConfig,
    plan: &crate::migration::ProviderSelectionPlan,
    selected_kind: mvp::config::ProviderKind,
) -> mvp::config::ProviderConfig {
    crate::migration::resolve_provider_config_from_selection(current_provider, plan, selected_kind)
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
