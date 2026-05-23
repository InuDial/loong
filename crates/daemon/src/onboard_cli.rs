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
pub use self::guided_config_support::resolve_guided_prompt_path_label_for_test;
use self::guided_config_support::*;
#[path = "onboard_entry_render.rs"]
mod entry_render_support;
#[path = "onboard_flow_support.rs"]
mod flow_support;
#[path = "onboard_flow_types.rs"]
mod flow_types_support;
#[path = "onboard_guided_render.rs"]
mod guided_render_support;
#[path = "onboard_cli_render.rs"]
mod onboard_cli_render;
#[path = "onboard_review_render.rs"]
mod onboard_review_render;
#[path = "onboard_prompt_ui.rs"]
mod prompt_ui_support;
#[path = "onboard_provider_selection.rs"]
mod provider_selection_support;
#[path = "onboard_runtime.rs"]
mod runtime_support;
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
use self::flow_types_support::*;
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
use self::provider_selection_support::*;
pub use self::provider_selection_support::{
    build_provider_selection_plan_for_candidate, resolve_provider_config_from_selection,
    resolve_provider_config_from_selector,
};
pub use self::runtime_support::{
    OnboardCommandOptions, OnboardRuntimeContext, OnboardUi, SelectInteractionMode, SelectOption,
    run_onboard_cli, run_onboard_cli_with_ui,
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

#[cfg(test)]
fn provider_model_probe_failure_check(
    config: &mvp::config::LoongConfig,
    error: String,
) -> OnboardCheck {
    runtime_support::provider_model_probe_failure_check(config, error)
}

pub type ChannelImportReadiness = crate::migration::ChannelImportReadiness;

#[cfg(test)]
#[path = "onboard_cli_tests.rs"]
mod tests;
