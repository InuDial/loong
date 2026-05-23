use std::collections::BTreeSet;
use std::env;
#[cfg(test)]
use std::io;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::thread;
use std::time::Duration;

use loong_app as mvp;
use loong_contracts::SecretRef;
use loong_spec::CliResult;

use crate::copilot_onboarding::finalize_github_copilot_onboard_credentials;
#[cfg(not(test))]
use crate::onboard_finalize::OnboardWriteRecovery;
use crate::onboard_finalize::{
    ConfigWritePlan, build_onboarding_success_summary_with_memory, prepare_output_path_for_write,
    render_onboarding_success_summary_lines, resolve_backup_path, rollback_onboard_write_failure,
};
#[cfg(test)]
use crate::onboard_finalize::{
    OnboardWriteRecovery, format_backup_timestamp_at, resolve_backup_path_at,
};
pub use crate::onboard_preflight::{
    OnboardCheck, OnboardCheckLevel, OnboardNonInteractiveWarningPolicy,
    collect_channel_preflight_checks, directory_preflight_check, provider_credential_check,
    render_current_setup_preflight_summary_screen_lines,
    render_detected_setup_preflight_summary_screen_lines, render_preflight_summary_screen_lines,
};
use crate::onboard_preflight::{
    config_validation_failure_message,
    is_explicitly_accepted_non_interactive_warning as preflight_accepts_non_interactive_warning,
    non_interactive_preflight_failure_message, render_preflight_summary_screen_lines_with_progress,
    run_preflight_checks,
};
pub use crate::onboard_types::OnboardingCredentialSummary;
#[cfg(test)]
use crate::onboard_web_search::{
    WebSearchProviderRecommendation, WebSearchProviderRecommendationSource,
    recommend_web_search_provider_from_available_credentials,
};
use crate::onboard_web_search::{
    current_web_search_provider, explicit_web_search_provider_override,
    resolve_effective_web_search_default_provider, resolve_web_search_provider_recommendation,
};
use crate::onboarding_model_policy;
use crate::provider_credential_policy;
use crate::query_search_guidance::{
    configured_query_search_credential_env_name, configured_query_search_credential_source_value,
    configured_query_search_secret, preferred_query_search_credential_env_default,
    query_search_has_inline_credential, query_search_provider_display_name,
    summarize_query_search_credential,
};
use mvp::tui_surface::{
    TuiCalloutTone, TuiChoiceSpec, TuiHeaderStyle, TuiScreenSpec, TuiSectionSpec,
    render_onboard_screen_spec,
};
#[cfg(test)]
use std::fs;
#[cfg(test)]
use time::OffsetDateTime;

#[path = "onboard_select.rs"]
mod select_support;

pub use crate::onboard_import::{
    ImportCandidate, ImportSurface, ImportSurfaceLevel, OnboardEntryChoice, OnboardEntryOption,
    build_onboard_entry_options,
};
use crate::onboard_import::{
    StartingConfigSelection, default_onboard_entry_choice, default_starting_config_selection,
    import_candidate_from_migration, migration_candidate_for_onboard_display,
    migration_candidate_from_onboard, onboard_starting_point_label, prepare_import_starting_state,
    select_non_interactive_starting_config_from_state, sort_starting_point_candidates,
};

use self::select_support::*;
#[path = "onboard_screen_specs.rs"]
mod screen_spec_support;

use self::screen_spec_support::*;
#[path = "onboard_starting_point_render.rs"]
mod starting_point_render_support;
#[path = "onboard_starting_point.rs"]
mod starting_point_support;
use self::starting_point_support::*;
pub use self::starting_point_support::{
    collect_import_candidates_with_paths, detect_import_starting_config_with_channel_readiness,
    should_offer_current_setup_shortcut, should_offer_detected_setup_shortcut,
    validate_non_interactive_risk_gate,
};
#[path = "onboard_preinstalled_skills.rs"]
mod preinstalled_skills_support;
use self::preinstalled_skills_support::*;
#[path = "onboard_tail_support.rs"]
mod tail_support;
use self::tail_support::*;
pub use self::tail_support::{
    build_channel_onboarding_follow_up_lines, collect_import_surfaces,
    collect_import_surfaces_with_channel_readiness, memory_profile_id, parse_memory_profile,
    parse_prompt_personality, parse_provider_kind, preferred_api_key_env_default,
    prompt_personality_id, provider_default_api_key_env, provider_kind_display_name,
    provider_kind_id, should_skip_config_write, supported_memory_profile_list,
    supported_personality_list, supported_provider_list,
};
#[path = "onboard_guided_config.rs"]
mod guided_config_support;
use self::guided_config_support::*;
pub use self::guided_config_support::{
    build_provider_selection_plan_for_candidate, resolve_provider_config_from_selection,
    resolve_provider_config_from_selector,
};
#[path = "onboard_entry_render.rs"]
mod entry_render_support;
#[path = "onboard_flow_support.rs"]
mod flow_support;
#[path = "onboard_guided_render.rs"]
mod guided_render_support;
#[path = "onboard_cli_render.rs"]
mod onboard_cli_render;
#[path = "onboard_review_render.rs"]
mod onboard_review_render;
#[path = "onboard_prompt_ui.rs"]
mod prompt_ui_support;
#[path = "onboard_shortcut_write_render.rs"]
mod shortcut_write_render_support;
pub use self::entry_render_support::render_onboard_entry_screen_lines;
use self::entry_render_support::{
    prompt_onboard_entry_choice, render_onboard_entry_interactive_screen_lines_with_style,
};
use self::flow_support::{
    OnboardSessionPreparation, build_onboard_review_context, complete_onboard_preflight,
    finalize_onboard_closeout, is_explicitly_accepted_non_interactive_warning,
    prepare_onboard_session,
};
pub use self::guided_render_support::{
    render_api_key_env_selection_screen_lines,
    render_api_key_env_selection_screen_lines_with_default, render_model_selection_screen_lines,
    render_model_selection_screen_lines_with_default, render_provider_selection_screen_lines,
    render_system_prompt_selection_screen_lines,
    render_system_prompt_selection_screen_lines_with_default,
};
use self::guided_render_support::{
    render_api_key_env_selection_screen_lines_with_style,
    render_model_selection_screen_lines_with_style, render_provider_selection_header_lines,
    render_system_prompt_selection_screen_lines_with_style,
    render_web_search_credential_selection_screen_lines_with_style,
};
pub use self::onboard_cli_render::{append_escape_cancel_hint, render_default_choice_footer_line};
use self::onboard_cli_render::{
    render_onboard_choice_screen, render_prompt_with_default_text, screen_subtitle,
    tui_header_style,
};
#[cfg(test)]
use self::onboard_cli_render::{render_onboard_option_lines, render_onboard_option_prefix};
#[cfg(test)]
use self::onboard_review_render::provider_matches_for_review;
use self::onboard_review_render::{
    build_onboard_review_candidate_with_selected_context,
    render_onboard_review_lines_with_guidance_and_style,
};
pub use self::onboard_review_render::{
    render_current_setup_review_lines_with_guidance,
    render_detected_setup_review_lines_with_guidance, render_onboard_review_lines_with_guidance,
    summarize_prompt_addendum, summarize_prompt_mode, summarize_provider_credential,
};
pub(crate) use self::prompt_ui_support::StdioOnboardUi;
#[cfg(test)]
use self::prompt_ui_support::{
    OnboardPromptLineReader, OnboardPromptRead, StdioOnboardLineMessage, StdioOnboardLineReader,
    is_explicit_onboard_cancel_input, onboard_line_channel_with_capacity,
    onboard_paste_drain_window, read_single_line_prompt_capture,
};
use self::prompt_ui_support::{
    ensure_onboard_input_not_cancelled, is_explicit_onboard_clear_input, print_lines, print_message,
};
#[cfg(test)]
use self::shortcut_write_render_support::render_onboard_shortcut_screen_lines_with_style;
pub use self::shortcut_write_render_support::{
    render_continue_current_setup_screen_lines, render_continue_detected_setup_screen_lines,
    render_current_setup_write_confirmation_screen_lines,
    render_detected_setup_write_confirmation_screen_lines,
    render_existing_config_write_screen_lines, render_onboarding_risk_screen_lines,
    render_write_confirmation_screen_lines,
};
use self::shortcut_write_render_support::{
    render_existing_config_write_header_lines_with_style,
    render_onboard_shortcut_header_lines_with_style,
};
#[cfg(test)]
use self::starting_point_render_support::render_starting_point_selection_header_lines_with_style;
pub use self::starting_point_render_support::{
    render_single_detected_setup_preview_screen_lines, render_starting_point_selection_screen_lines,
};
pub use crate::onboard_finalize::{
    OnboardingAction, OnboardingActionKind, OnboardingChannelSurfaceSummary,
    OnboardingDomainOutcome, OnboardingSuccessSummary, backup_existing_config,
    build_onboarding_success_summary, render_onboarding_success_summary_with_width,
};
const ONBOARD_CLEAR_INPUT_TOKEN: &str = ":clear";
const ONBOARD_CUSTOM_MODEL_OPTION_SLUG: &str = "__custom_model__";
const ONBOARD_ESCAPE_CANCEL_HINT: &str = "- press Esc then Enter to cancel onboarding";
const ONBOARD_SINGLE_LINE_INPUT_HINT: &str = "- single-line input only";
const ONBOARD_PASTE_DRAIN_WINDOW_ENV: &str = "LOONG_ONBOARD_PASTE_DRAIN_WINDOW_MS";
const DEFAULT_ONBOARD_PASTE_DRAIN_WINDOW: Duration = Duration::from_millis(75);
const ONBOARD_LINE_READER_BUFFER_SIZE: usize = 64;
const PREINSTALLED_SKILLS_PROMPT_LABEL: &str = "preinstalled skills";

#[derive(Debug, Clone)]
pub struct OnboardCommandOptions {
    pub output: Option<String>,
    pub force: bool,
    pub non_interactive: bool,
    pub accept_risk: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key_env: Option<String>,
    pub web_search_provider: Option<String>,
    pub web_search_api_key_env: Option<String>,
    pub personality: Option<String>,
    pub memory_profile: Option<String>,
    pub system_prompt: Option<String>,
    pub skip_model_probe: bool,
}

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub label: String,
    pub slug: String,
    pub description: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectInteractionMode {
    List,
    Search,
}

pub trait OnboardUi {
    fn print_line(&mut self, line: &str) -> CliResult<()>;
    fn prompt_with_default(&mut self, label: &str, default: &str) -> CliResult<String>;
    fn prompt_required(&mut self, label: &str) -> CliResult<String>;
    fn prompt_allow_empty(&mut self, label: &str) -> CliResult<String> {
        self.prompt_required(label)
    }
    fn prompt_confirm(&mut self, message: &str, default: bool) -> CliResult<bool>;
    fn select_one(
        &mut self,
        label: &str,
        options: &[SelectOption],
        default: Option<usize>,
        interaction_mode: SelectInteractionMode,
    ) -> CliResult<usize>;
}

#[derive(Debug, Clone)]
pub struct OnboardRuntimeContext {
    render_width: usize,
    workspace_root: Option<PathBuf>,
    codex_config_paths: Vec<PathBuf>,
}

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

impl OnboardRuntimeContext {
    fn capture() -> Self {
        Self {
            render_width: detect_render_width(),
            workspace_root: env::current_dir().ok(),
            codex_config_paths: default_codex_config_paths(),
        }
    }

    pub fn new_for_tests(
        render_width: usize,
        workspace_root: Option<PathBuf>,
        codex_config_paths: impl IntoIterator<Item = PathBuf>,
    ) -> Self {
        Self {
            render_width,
            workspace_root,
            codex_config_paths: codex_config_paths.into_iter().collect(),
        }
    }
}

#[cfg(test)]
fn provider_model_probe_failure_check(
    config: &mvp::config::LoongConfig,
    error: String,
) -> OnboardCheck {
    crate::onboard_preflight::provider_model_probe_failure_check(config, error)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardHeaderStyle {
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuidedPromptPath {
    NativePromptPack,
    InlineOverride,
}

impl GuidedPromptPath {
    const fn total_steps(self) -> usize {
        match self {
            GuidedPromptPath::NativePromptPack => 7,
            GuidedPromptPath::InlineOverride => 6,
        }
    }

    const fn index(self, step: GuidedOnboardStep) -> usize {
        match (self, step) {
            (_, GuidedOnboardStep::Provider) => 1,
            (_, GuidedOnboardStep::Model) => 2,
            (_, GuidedOnboardStep::CredentialEnv) => 3,
            (GuidedPromptPath::NativePromptPack, GuidedOnboardStep::PromptCustomization) => 4,
            (_, GuidedOnboardStep::WebSearchProvider) => match self {
                GuidedPromptPath::NativePromptPack => 5,
                GuidedPromptPath::InlineOverride => 4,
            },
            (GuidedPromptPath::NativePromptPack, GuidedOnboardStep::Review) => 6,
            (GuidedPromptPath::InlineOverride, GuidedOnboardStep::PromptCustomization) => 4,
            (GuidedPromptPath::InlineOverride, GuidedOnboardStep::Review) => 5,
        }
    }

    const fn label(self, step: GuidedOnboardStep) -> &'static str {
        match step {
            GuidedOnboardStep::Provider => "provider",
            GuidedOnboardStep::Model => "model",
            GuidedOnboardStep::CredentialEnv => "credential source",
            GuidedOnboardStep::PromptCustomization => match self {
                GuidedPromptPath::NativePromptPack => "prompt addendum",
                GuidedPromptPath::InlineOverride => "system prompt",
            },
            GuidedOnboardStep::WebSearchProvider => "web search",
            GuidedOnboardStep::Review => "review",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuidedOnboardStep {
    Provider,
    Model,
    CredentialEnv,
    PromptCustomization,
    WebSearchProvider,
    Review,
}

impl GuidedOnboardStep {
    fn progress_line(self, path: GuidedPromptPath) -> String {
        format!(
            "step {} of {} · {}",
            path.index(self),
            path.total_steps(),
            path.label(self)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewFlowStyle {
    Guided(GuidedPromptPath),
    QuickCurrentSetup,
    QuickDetectedSetup,
}

impl ReviewFlowStyle {
    const fn review_kind(self) -> crate::onboard_presentation::ReviewFlowKind {
        match self {
            ReviewFlowStyle::Guided(_) => crate::onboard_presentation::ReviewFlowKind::Guided,
            ReviewFlowStyle::QuickCurrentSetup => {
                crate::onboard_presentation::ReviewFlowKind::QuickCurrentSetup
            }
            ReviewFlowStyle::QuickDetectedSetup => {
                crate::onboard_presentation::ReviewFlowKind::QuickDetectedSetup
            }
        }
    }

    fn progress_line(self) -> String {
        match self {
            ReviewFlowStyle::Guided(prompt_path) => {
                GuidedOnboardStep::Review.progress_line(prompt_path)
            }
            ReviewFlowStyle::QuickCurrentSetup | ReviewFlowStyle::QuickDetectedSetup => {
                crate::onboard_presentation::review_flow_copy(self.review_kind())
                    .progress_line
                    .to_owned()
            }
        }
    }

    const fn header_subtitle(self) -> &'static str {
        crate::onboard_presentation::review_flow_copy(self.review_kind()).header_subtitle
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OnboardScreenOption {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) detail_lines: Vec<String>,
    pub(crate) recommended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WebSearchCredentialSelection {
    KeepCurrent,
    ClearConfigured,
    UseEnv(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartingPointFitHint {
    key: &'static str,
    detail: String,
    domain: Option<crate::migration::SetupDomainKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardShortcutKind {
    CurrentSetup,
    DetectedSetup,
}

impl OnboardShortcutKind {
    const fn presentation_kind(self) -> crate::onboard_presentation::ShortcutKind {
        match self {
            OnboardShortcutKind::CurrentSetup => {
                crate::onboard_presentation::ShortcutKind::CurrentSetup
            }
            OnboardShortcutKind::DetectedSetup => {
                crate::onboard_presentation::ShortcutKind::DetectedSetup
            }
        }
    }

    const fn review_flow_style(self) -> ReviewFlowStyle {
        match self {
            OnboardShortcutKind::CurrentSetup => ReviewFlowStyle::QuickCurrentSetup,
            OnboardShortcutKind::DetectedSetup => ReviewFlowStyle::QuickDetectedSetup,
        }
    }

    const fn subtitle(self) -> &'static str {
        crate::onboard_presentation::shortcut_copy(self.presentation_kind()).subtitle
    }

    const fn title(self) -> &'static str {
        crate::onboard_presentation::shortcut_copy(self.presentation_kind()).title
    }

    const fn summary_line(self) -> &'static str {
        crate::onboard_presentation::shortcut_copy(self.presentation_kind()).summary_line
    }

    const fn primary_label(self) -> &'static str {
        crate::onboard_presentation::shortcut_copy(self.presentation_kind()).primary_label
    }

    const fn default_choice_description(self) -> &'static str {
        crate::onboard_presentation::shortcut_copy(self.presentation_kind())
            .default_choice_description
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnboardShortcutChoice {
    UseShortcut,
    AdjustSettings,
}
pub type ChannelImportReadiness = crate::migration::ChannelImportReadiness;

pub async fn run_onboard_cli(options: OnboardCommandOptions) -> CliResult<()> {
    let context = OnboardRuntimeContext::capture();
    let mut ui = StdioOnboardUi::default();
    run_onboard_cli_with_ui(options, &mut ui, &context).await
}

pub async fn run_onboard_cli_with_ui(
    options: OnboardCommandOptions,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<()> {
    let preparation = prepare_onboard_session(&options, ui, context)?;
    let OnboardSessionPreparation {
        output_path,
        starting_selection,
        mut config,
        skip_detailed_setup,
        review_flow_style,
    } = preparation;
    let reuse_existing_non_interactive_config = options.non_interactive
        && starting_selection.entry_choice == OnboardEntryChoice::ContinueCurrentSetup
        && starting_selection.current_setup_state == crate::migration::CurrentSetupState::Healthy
        && !onboard_has_explicit_overrides(&options);

    if !skip_detailed_setup && !reuse_existing_non_interactive_config {
        apply_guided_onboard_configuration(
            &options,
            &output_path,
            &starting_selection.provider_selection,
            &mut config,
            ui,
            context,
        )
        .await?;
    }
    let selected_preinstalled_skill_ids =
        resolve_preinstalled_skill_selection(&options, ui, context)?;
    apply_selected_preinstalled_skills_to_config(
        &mut config,
        &output_path,
        &selected_preinstalled_skill_ids,
    );

    let review =
        build_onboard_review_context(&config, &starting_selection, review_flow_style, context);
    let preflight = complete_onboard_preflight(
        &options,
        &output_path,
        &config,
        reuse_existing_non_interactive_config,
        review.review_flow_style,
        ui,
        context,
    )
    .await?;
    finalize_onboard_closeout(
        &options,
        &output_path,
        &config,
        &selected_preinstalled_skill_ids,
        &starting_selection,
        &review,
        preflight,
        ui,
        context,
    )
    .await
}

async fn apply_guided_onboard_configuration(
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

fn resolve_guided_prompt_path(
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

#[cfg(test)]
#[path = "onboard_cli_tests.rs"]
mod tests;
