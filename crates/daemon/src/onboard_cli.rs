use std::collections::BTreeSet;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use dialoguer::console::{Term, user_attended};
use dialoguer::theme::ColorfulTheme;
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
#[path = "onboard_guided_config.rs"]
mod guided_config_support;
use self::guided_config_support::*;
pub use self::guided_config_support::{
    build_provider_selection_plan_for_candidate, resolve_provider_config_from_selection,
    resolve_provider_config_from_selector,
};
#[path = "onboard_cli_render.rs"]
mod onboard_cli_render;
#[path = "onboard_review_render.rs"]
mod onboard_review_render;
pub use self::onboard_cli_render::{
    append_escape_cancel_hint, render_api_key_env_selection_screen_lines,
    render_api_key_env_selection_screen_lines_with_default,
    render_continue_current_setup_screen_lines, render_continue_detected_setup_screen_lines,
    render_current_setup_write_confirmation_screen_lines, render_default_choice_footer_line,
    render_detected_setup_write_confirmation_screen_lines,
    render_existing_config_write_screen_lines, render_model_selection_screen_lines,
    render_model_selection_screen_lines_with_default, render_onboard_entry_screen_lines,
    render_onboarding_risk_screen_lines, render_provider_selection_screen_lines,
    render_system_prompt_selection_screen_lines,
    render_system_prompt_selection_screen_lines_with_default,
    render_write_confirmation_screen_lines,
};
use self::onboard_cli_render::{
    prompt_onboard_entry_choice, render_api_key_env_selection_screen_lines_with_style,
    render_existing_config_write_header_lines_with_style,
    render_model_selection_screen_lines_with_style, render_onboard_choice_screen,
    render_onboard_entry_interactive_screen_lines_with_style,
    render_onboard_shortcut_header_lines_with_style, render_prompt_with_default_text,
    render_provider_selection_header_lines, render_system_prompt_selection_screen_lines_with_style,
    render_web_search_credential_selection_screen_lines_with_style, screen_subtitle,
    tui_header_style,
};
#[cfg(test)]
use self::onboard_cli_render::{
    render_onboard_option_lines, render_onboard_option_prefix,
    render_onboard_shortcut_screen_lines_with_style,
};
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

struct OnboardSessionPreparation {
    output_path: PathBuf,
    starting_selection: StartingConfigSelection,
    config: mvp::config::LoongConfig,
    skip_detailed_setup: bool,
    review_flow_style: ReviewFlowStyle,
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

struct OnboardReviewContext {
    review_candidate: crate::migration::ImportCandidate,
    review_flow_style: ReviewFlowStyle,
}

struct OnboardPreflightResult {
    checks: Vec<OnboardCheck>,
    skip_config_write: bool,
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

fn is_explicitly_accepted_non_interactive_warning(
    check: &OnboardCheck,
    options: &OnboardCommandOptions,
) -> bool {
    preflight_accepts_non_interactive_warning(check, options.skip_model_probe)
}

#[cfg(test)]
fn provider_model_probe_failure_check(
    config: &mvp::config::LoongConfig,
    error: String,
) -> OnboardCheck {
    crate::onboard_preflight::provider_model_probe_failure_check(config, error)
}

trait OnboardPromptLineReader {
    fn read_blocking_line(&mut self) -> CliResult<OnboardPromptRead>;
    fn read_pending_line(&mut self) -> CliResult<Option<String>>;
}

#[derive(Debug, PartialEq, Eq)]
enum OnboardPromptRead {
    Line(String),
    Eof,
}

#[derive(Debug)]
enum StdioOnboardLineMessage {
    Line(String),
    Eof,
    Error(String),
}

type StdioOnboardLineSender = mpsc::SyncSender<StdioOnboardLineMessage>;

#[derive(Debug)]
enum StdioOnboardLineReader {
    Background {
        receiver: Receiver<StdioOnboardLineMessage>,
        paste_drain_window: Duration,
    },
    Direct {
        degraded_notice: Option<String>,
    },
}

fn onboard_line_channel() -> (StdioOnboardLineSender, Receiver<StdioOnboardLineMessage>) {
    onboard_line_channel_with_capacity(ONBOARD_LINE_READER_BUFFER_SIZE)
}

fn onboard_line_channel_with_capacity(
    buffer_size: usize,
) -> (StdioOnboardLineSender, Receiver<StdioOnboardLineMessage>) {
    assert!(
        buffer_size > 0,
        "onboard line reader buffer must be non-zero"
    );
    mpsc::sync_channel(buffer_size)
}

fn onboard_paste_drain_window() -> Duration {
    env::var(ONBOARD_PASTE_DRAIN_WINDOW_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_ONBOARD_PASTE_DRAIN_WINDOW)
}

fn spawn_onboard_stdin_reader(sender: StdioOnboardLineSender) -> io::Result<()> {
    thread::Builder::new()
        .name("loong-onboard-stdin".to_owned())
        .spawn(move || {
            loop {
                let mut line = String::new();
                match io::stdin().read_line(&mut line) {
                    Ok(0) => {
                        let _ = sender.send(StdioOnboardLineMessage::Eof);
                        break;
                    }
                    Ok(_) => {
                        if sender.send(StdioOnboardLineMessage::Line(line)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(StdioOnboardLineMessage::Error(format!(
                            "read stdin failed: {error}"
                        )));
                        break;
                    }
                }
            }
        })
        .map(|_handle| ())
}

fn format_onboard_line_reader_spawn_notice(error: &io::Error) -> String {
    format!(
        "warning: failed to start onboarding stdin reader thread ({error}); single-line paste draining is disabled for this session"
    )
}

impl StdioOnboardLineReader {
    fn background_from_receiver(receiver: Receiver<StdioOnboardLineMessage>) -> Self {
        Self::Background {
            receiver,
            paste_drain_window: onboard_paste_drain_window(),
        }
    }

    fn try_spawn_background_receiver() -> io::Result<Receiver<StdioOnboardLineMessage>> {
        let (sender, receiver) = onboard_line_channel();
        spawn_onboard_stdin_reader(sender)?;
        Ok(receiver)
    }

    fn from_spawn_result(result: io::Result<Receiver<StdioOnboardLineMessage>>) -> Self {
        match result {
            Ok(receiver) => Self::background_from_receiver(receiver),
            Err(error) => Self::Direct {
                degraded_notice: Some(format_onboard_line_reader_spawn_notice(&error)),
            },
        }
    }

    fn take_degraded_notice(&mut self) -> Option<String> {
        match self {
            Self::Background { .. } => None,
            Self::Direct { degraded_notice } => degraded_notice.take(),
        }
    }
}

impl Default for StdioOnboardLineReader {
    fn default() -> Self {
        Self::from_spawn_result(Self::try_spawn_background_receiver())
    }
}

impl OnboardPromptLineReader for StdioOnboardLineReader {
    fn read_blocking_line(&mut self) -> CliResult<OnboardPromptRead> {
        if let Some(notice) = self.take_degraded_notice() {
            eprintln!("{notice}");
        }
        match self {
            Self::Background { receiver, .. } => match receiver.recv() {
                Ok(StdioOnboardLineMessage::Line(line)) => Ok(OnboardPromptRead::Line(line)),
                Ok(StdioOnboardLineMessage::Eof) => Ok(OnboardPromptRead::Eof),
                Ok(StdioOnboardLineMessage::Error(error)) => Err(error),
                Err(_) => Ok(OnboardPromptRead::Eof),
            },
            Self::Direct { .. } => {
                let mut line = String::new();
                let bytes_read = io::stdin()
                    .read_line(&mut line)
                    .map_err(|error| format!("read stdin failed: {error}"))?;
                if bytes_read == 0 {
                    return Ok(OnboardPromptRead::Eof);
                }
                Ok(OnboardPromptRead::Line(line))
            }
        }
    }

    fn read_pending_line(&mut self) -> CliResult<Option<String>> {
        match self {
            Self::Background {
                receiver,
                paste_drain_window,
            } => match receiver.recv_timeout(*paste_drain_window) {
                Ok(StdioOnboardLineMessage::Line(line)) => Ok(Some(line)),
                Ok(StdioOnboardLineMessage::Eof) => Ok(None),
                Ok(StdioOnboardLineMessage::Error(error)) => Err(error),
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => Ok(None),
            },
            Self::Direct { .. } => Ok(None),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct StdioOnboardUi {
    line_reader: Option<StdioOnboardLineReader>,
}

impl StdioOnboardUi {
    fn stdio_line_reader(&mut self) -> &mut StdioOnboardLineReader {
        self.line_reader
            .get_or_insert_with(StdioOnboardLineReader::default)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct OnboardPromptCapture {
    raw: String,
    dropped_line_count: usize,
    reached_eof: bool,
}

fn read_single_line_prompt_capture(
    reader: &mut impl OnboardPromptLineReader,
) -> CliResult<OnboardPromptCapture> {
    let read = reader.read_blocking_line()?;
    let mut dropped_line_count = 0;
    let (raw, reached_eof) = match read {
        OnboardPromptRead::Line(raw) => {
            while reader.read_pending_line()?.is_some() {
                dropped_line_count += 1;
            }
            (raw, false)
        }
        OnboardPromptRead::Eof => (String::new(), true),
    };
    Ok(OnboardPromptCapture {
        raw,
        dropped_line_count,
        reached_eof,
    })
}

fn print_dropped_paste_notice(label: &str, dropped_line_count: usize) {
    if dropped_line_count == 0 {
        return;
    }
    let noun = if dropped_line_count == 1 {
        "line"
    } else {
        "lines"
    };
    println!(
        "note: {label} accepts a single line; ignored {dropped_line_count} extra pasted {noun}"
    );
}

impl OnboardUi for StdioOnboardUi {
    fn print_line(&mut self, line: &str) -> CliResult<()> {
        println!("{line}");
        Ok(())
    }

    fn prompt_with_default(&mut self, label: &str, default: &str) -> CliResult<String> {
        if rich_prompt_ui_available() {
            return prompt_with_default_rich(label, default);
        }
        prompt_with_default_stdio(self.stdio_line_reader(), label, default)
    }

    fn prompt_required(&mut self, label: &str) -> CliResult<String> {
        if rich_prompt_ui_available() {
            return prompt_required_rich(label);
        }
        prompt_required_stdio(self.stdio_line_reader(), label)
    }

    fn prompt_allow_empty(&mut self, label: &str) -> CliResult<String> {
        if rich_prompt_ui_available() {
            return prompt_allow_empty_rich(label);
        }
        prompt_required_stdio(self.stdio_line_reader(), label)
    }

    fn prompt_confirm(&mut self, message: &str, default: bool) -> CliResult<bool> {
        if rich_prompt_ui_available() {
            return prompt_confirm_rich(message, default);
        }
        prompt_confirm_stdio(self.stdio_line_reader(), message, default)
    }

    fn select_one(
        &mut self,
        label: &str,
        options: &[SelectOption],
        default: Option<usize>,
        interaction_mode: SelectInteractionMode,
    ) -> CliResult<usize> {
        if rich_prompt_ui_available() {
            return select_one_rich(label, options, default, interaction_mode);
        }
        select_one_stdio(self.stdio_line_reader(), label, options, default)
    }
}

fn prompt_with_default_stdio(
    line_reader: &mut impl OnboardPromptLineReader,
    label: &str,
    default: &str,
) -> CliResult<String> {
    print!("{}", render_prompt_with_default_text(label, default));
    io::stdout()
        .flush()
        .map_err(|error| format!("flush stdout failed: {error}"))?;
    let capture = read_single_line_prompt_capture(line_reader)?;
    let line = ensure_onboard_input_not_cancelled(capture.raw)?;
    print_dropped_paste_notice(label, capture.dropped_line_count);
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(default.to_owned());
    }
    Ok(trimmed.to_owned())
}

fn prompt_required_stdio(
    line_reader: &mut impl OnboardPromptLineReader,
    label: &str,
) -> CliResult<String> {
    print!("{label}: ");
    io::stdout()
        .flush()
        .map_err(|error| format!("flush stdout failed: {error}"))?;
    let capture = read_single_line_prompt_capture(line_reader)?;
    let line = ensure_onboard_input_not_cancelled(capture.raw)?;
    print_dropped_paste_notice(label, capture.dropped_line_count);
    Ok(line.trim().to_owned())
}

fn prompt_confirm_stdio(
    line_reader: &mut impl OnboardPromptLineReader,
    message: &str,
    default: bool,
) -> CliResult<bool> {
    let suffix = if default { "[Y/n]" } else { "[y/N]" };
    print!("{message} {suffix}: ");
    io::stdout()
        .flush()
        .map_err(|error| format!("flush stdout failed: {error}"))?;
    let capture = read_single_line_prompt_capture(line_reader)?;
    let line = ensure_onboard_input_not_cancelled(capture.raw)?;
    print_dropped_paste_notice(message, capture.dropped_line_count);
    let value = line.trim().to_ascii_lowercase();
    if value.is_empty() {
        return Ok(default);
    }
    Ok(matches!(value.as_str(), "y" | "yes"))
}

fn select_one_stdio(
    line_reader: &mut impl OnboardPromptLineReader,
    label: &str,
    options: &[SelectOption],
    default: Option<usize>,
) -> CliResult<usize> {
    let default = validate_select_one_state(options.len(), default)?;
    loop {
        for (i, opt) in options.iter().enumerate() {
            let num = i + 1;
            let rec = if opt.recommended {
                " (recommended)"
            } else {
                ""
            };
            println!("  {num}) {}{rec}", opt.label);
            if !opt.description.is_empty() {
                println!("     {}", opt.description);
            }
        }
        println!();
        let prompt_text = match default {
            Some(idx) => format!("{label} (default {}):", idx + 1),
            None => format!("{label}: "),
        };
        print!("{prompt_text}");
        io::stdout()
            .flush()
            .map_err(|error| format!("flush stdout failed: {error}"))?;
        let capture = read_single_line_prompt_capture(line_reader)?;
        print_dropped_paste_notice(label, capture.dropped_line_count);
        if capture.reached_eof {
            return resolve_select_one_eof(default);
        }
        let input = ensure_onboard_input_not_cancelled(capture.raw)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            if let Some(idx) = default {
                return Ok(idx);
            }
            println!("Please select an option.");
            continue;
        }
        if let Some(index) = parse_select_one_input(trimmed, options) {
            return Ok(index);
        }
        println!("{}", render_select_one_invalid_input_message(options));
    }
}

fn rich_prompt_ui_available() -> bool {
    user_attended()
}

fn rich_prompt_theme() -> ColorfulTheme {
    ColorfulTheme::default()
}

fn rich_prompt_term() -> Term {
    Term::stdout()
}

fn print_lines(ui: &mut impl OnboardUi, lines: impl IntoIterator<Item = String>) -> CliResult<()> {
    for line in lines {
        ui.print_line(&line)?;
    }
    Ok(())
}

fn print_message(ui: &mut impl OnboardUi, line: impl Into<String>) -> CliResult<()> {
    ui.print_line(&line.into())
}

fn is_explicit_onboard_clear_input(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case(ONBOARD_CLEAR_INPUT_TOKEN)
}

fn is_explicit_onboard_cancel_input(raw: &str) -> bool {
    matches!(raw.trim(), "\u{1b}")
}

fn ensure_onboard_input_not_cancelled(raw: String) -> CliResult<String> {
    if is_explicit_onboard_cancel_input(raw.as_str()) {
        return Err("onboarding cancelled: escape input received".to_owned());
    }
    Ok(raw)
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

fn prepare_onboard_session(
    options: &OnboardCommandOptions,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<OnboardSessionPreparation> {
    acknowledge_onboard_risk(options, ui, context)?;

    let output_path = options
        .output
        .as_deref()
        .map(mvp::config::expand_path)
        .unwrap_or_else(mvp::config::default_config_path);
    let starting_selection = load_import_starting_config(&output_path, options, ui, context)?;
    let (config, skip_detailed_setup, review_flow_style) =
        prepare_onboard_starting_selection(options, &starting_selection, ui, context)?;

    Ok(OnboardSessionPreparation {
        output_path,
        starting_selection,
        config,
        skip_detailed_setup,
        review_flow_style,
    })
}

fn acknowledge_onboard_risk(
    options: &OnboardCommandOptions,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<()> {
    validate_non_interactive_risk_gate(options.non_interactive, options.accept_risk)?;

    if !options.non_interactive && !options.accept_risk {
        print_lines(
            ui,
            render_onboarding_risk_screen_lines_with_style(context.render_width, true),
        )?;
        if !ui.prompt_confirm(
            crate::onboard_presentation::risk_screen_copy().confirm_prompt,
            false,
        )? {
            return Err("onboarding cancelled: risk acknowledgement declined".to_owned());
        }
    }

    Ok(())
}

fn prepare_onboard_starting_selection(
    options: &OnboardCommandOptions,
    starting_selection: &StartingConfigSelection,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<(mvp::config::LoongConfig, bool, ReviewFlowStyle)> {
    let shortcut_kind = resolve_onboard_shortcut_kind(options, starting_selection);
    let config = starting_selection.config.clone();
    let skip_detailed_setup = if let Some(shortcut_kind) = shortcut_kind {
        print_lines(
            ui,
            render_onboard_shortcut_header_lines_with_style(
                shortcut_kind,
                &config,
                starting_selection.import_source.as_deref(),
                context.render_width,
                true,
            ),
        )?;
        matches!(
            prompt_onboard_shortcut_choice(ui, shortcut_kind)?,
            OnboardShortcutChoice::UseShortcut
        )
    } else {
        false
    };
    let review_flow_style = if skip_detailed_setup {
        shortcut_kind
            .map(OnboardShortcutKind::review_flow_style)
            .unwrap_or(ReviewFlowStyle::Guided(GuidedPromptPath::NativePromptPack))
    } else {
        ReviewFlowStyle::Guided(resolve_guided_prompt_path(options, &config))
    };

    Ok((config, skip_detailed_setup, review_flow_style))
}

fn build_onboard_review_context(
    config: &mvp::config::LoongConfig,
    starting_selection: &StartingConfigSelection,
    review_flow_style: ReviewFlowStyle,
    context: &OnboardRuntimeContext,
) -> OnboardReviewContext {
    let workspace_guidance = context
        .workspace_root
        .as_deref()
        .map(crate::migration::detect_workspace_guidance)
        .unwrap_or_default();
    let review_candidate = build_onboard_review_candidate_with_selected_context(
        config,
        &workspace_guidance,
        starting_selection.review_candidate.as_ref(),
    );
    OnboardReviewContext {
        review_candidate,
        review_flow_style,
    }
}

async fn complete_onboard_preflight(
    options: &OnboardCommandOptions,
    output_path: &Path,
    config: &mvp::config::LoongConfig,
    reuse_existing_non_interactive_config: bool,
    review_flow_style: ReviewFlowStyle,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<OnboardPreflightResult> {
    let checks = run_preflight_checks(config, options.skip_model_probe).await;
    let config_validation_failure = config_validation_failure_message(&checks);
    let credential_ok = checks
        .iter()
        .find(|check| check.name == "provider credentials")
        .is_some_and(|check| check.level == OnboardCheckLevel::Pass);
    let has_failures = checks
        .iter()
        .any(|check| check.level == OnboardCheckLevel::Fail);
    let has_warnings = checks
        .iter()
        .any(|check| check.level == OnboardCheckLevel::Warn);
    let existing_output_config = load_existing_output_config(output_path);
    let skip_config_write = reuse_existing_non_interactive_config
        || should_skip_config_write(existing_output_config.as_ref(), config);
    let has_blocking_non_interactive_warnings = !skip_config_write
        && checks.iter().any(|check| {
            check.level == OnboardCheckLevel::Warn
                && !is_explicitly_accepted_non_interactive_warning(check, options)
        });

    if options.non_interactive {
        if let Some(message) = config_validation_failure {
            return Err(message);
        }
        if !skip_config_write {
            if !credential_ok {
                let credential_hint =
                    provider_credential_policy::provider_credential_env_hint(&config.provider)
                        .unwrap_or_else(|| "PROVIDER_API_KEY".to_owned());
                return Err(format!(
                    "onboard preflight failed: provider credentials missing. configure inline credentials or set {} in env",
                    credential_hint
                ));
            }
            if has_failures {
                return Err(non_interactive_preflight_failure_message(&checks));
            }
            if has_blocking_non_interactive_warnings {
                let warning_message = non_interactive_preflight_warning_message(&checks, options);
                return Err(warning_message);
            }
        }
    } else {
        print_lines(
            ui,
            render_preflight_summary_screen_lines_with_style(
                &checks,
                context.render_width,
                review_flow_style,
                true,
            ),
        )?;
        if let Some(message) = config_validation_failure {
            return Err(message);
        }
        if (has_failures || has_warnings)
            && !ui.prompt_confirm(
                crate::onboard_presentation::preflight_confirm_prompt(),
                false,
            )?
        {
            return Err("onboarding cancelled: unresolved preflight warnings".to_owned());
        }
    }

    Ok(OnboardPreflightResult {
        checks,
        skip_config_write,
    })
}

async fn finalize_onboard_closeout(
    options: &OnboardCommandOptions,
    output_path: &Path,
    config: &mvp::config::LoongConfig,
    selected_preinstalled_skill_ids: &[String],
    starting_selection: &StartingConfigSelection,
    review: &OnboardReviewContext,
    preflight: OnboardPreflightResult,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<()> {
    let has_failures = preflight
        .checks
        .iter()
        .any(|check| check.level == OnboardCheckLevel::Fail);
    let has_warnings = preflight
        .checks
        .iter()
        .any(|check| check.level == OnboardCheckLevel::Warn);

    let workspace_guidance = context
        .workspace_root
        .as_deref()
        .map(crate::migration::detect_workspace_guidance)
        .unwrap_or_default();
    if !options.non_interactive {
        print_lines(
            ui,
            render_onboard_review_lines_with_guidance_and_style(
                config,
                starting_selection.import_source.as_deref(),
                &workspace_guidance,
                starting_selection.review_candidate.as_ref(),
                context.render_width,
                review.review_flow_style,
                true,
            ),
        )?;
    }

    if !options.non_interactive && !preflight.skip_config_write {
        print_lines(
            ui,
            render_write_confirmation_screen_lines_with_style(
                &output_path.display().to_string(),
                has_failures || has_warnings,
                context.render_width,
                review.review_flow_style,
                true,
            ),
        )?;
        if !ui.prompt_confirm(
            crate::onboard_presentation::write_confirmation_prompt(),
            true,
        )? {
            return Err("onboarding cancelled: review declined before write".to_owned());
        }
    }

    let (path, config_status, write_recovery): (
        PathBuf,
        Option<String>,
        Option<OnboardWriteRecovery>,
    ) = if preflight.skip_config_write {
        (
            output_path.to_path_buf(),
            Some("existing config kept; no changes were needed".to_owned()),
            None,
        )
    } else {
        let write_plan = resolve_write_plan(output_path, options, ui, context)?;
        let write_recovery = prepare_output_path_for_write(output_path, &write_plan)?;
        let backup_path = if write_recovery.keep_backup_on_success {
            write_recovery.backup_path.as_deref()
        } else {
            None
        };
        if let Some(backup_path) = backup_path {
            let backup_message = format!("Backed up existing config to: {}", backup_path.display());
            print_message(ui, backup_message)?;
        }
        let path = match mvp::config::write(options.output.as_deref(), config, write_plan.force) {
            Ok(path) => path,
            Err(error) => {
                return Err(rollback_onboard_write_failure(
                    output_path,
                    &write_recovery,
                    error,
                ));
            }
        };
        (path, None, Some(write_recovery))
    };

    #[cfg(feature = "memory-sqlite")]
    let memory_path = {
        let mem_config =
            mvp::memory::runtime_config::MemoryRuntimeConfig::from_memory_config(&config.memory);
        match mvp::memory::ensure_memory_db_ready(
            Some(config.memory.resolved_sqlite_path()),
            &mem_config,
        ) {
            Ok(path) => path,
            Err(error) => {
                let failure = format!("failed to bootstrap sqlite memory: {error}");
                if let Some(write_recovery) = write_recovery.as_ref() {
                    return Err(rollback_onboard_write_failure(
                        output_path,
                        write_recovery,
                        failure,
                    ));
                }
                return Err(failure);
            }
        }
    };

    let memory_path_display = Some(memory_path.display().to_string());
    #[cfg(not(feature = "memory-sqlite"))]
    let memory_path_display: Option<String> = None;

    if let Err(error) =
        install_selected_preinstalled_skills(&path, config, selected_preinstalled_skill_ids)
    {
        if let Some(write_recovery) = write_recovery.as_ref() {
            return Err(rollback_onboard_write_failure(
                output_path,
                write_recovery,
                error,
            ));
        }
        return Err(error);
    }

    if let Some(write_recovery) = write_recovery.as_ref() {
        write_recovery.finish_success();
    }

    let success_summary = build_onboarding_success_summary_with_memory(
        &path,
        config,
        starting_selection.import_source.as_deref(),
        Some(&review.review_candidate),
        memory_path_display.as_deref(),
        config_status.as_deref(),
    );
    let success_summary_lines =
        render_onboarding_success_summary_lines(&success_summary, context.render_width, true);
    print_lines(ui, success_summary_lines)?;
    Ok(())
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

pub fn build_channel_onboarding_follow_up_lines(config: &mvp::config::LoongConfig) -> Vec<String> {
    let inventory = mvp::channel::channel_inventory(config);
    let mut lines = Vec::with_capacity(inventory.channel_surfaces.len() + 1);
    lines.push("channel next steps:".to_owned());

    for surface in inventory.channel_surfaces {
        let aliases = if surface.catalog.aliases.is_empty() {
            "-".to_owned()
        } else {
            surface.catalog.aliases.join(",")
        };
        let repair_command = surface
            .catalog
            .onboarding
            .repair_command
            .map(|command| format!("\"{command}\""))
            .unwrap_or_else(|| "-".to_owned());
        lines.push(format!(
            "- {} [{}] selection_order={} selection_label=\"{}\" strategy={} aliases={} status_command=\"{}\" repair_command={} setup_hint=\"{}\" blurb=\"{}\"",
            surface.catalog.label,
            surface.catalog.id,
            surface.catalog.selection_order,
            surface.catalog.selection_label,
            surface.catalog.onboarding.strategy.as_str(),
            aliases,
            surface.catalog.onboarding.status_command,
            repair_command,
            surface.catalog.onboarding.setup_hint,
            surface.catalog.blurb,
        ));
    }

    lines
}

fn normalize_onboard_credential_env_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if !onboard_credential_env_name_is_safe(trimmed) {
        return None;
    }

    Some(trimmed.to_owned())
}

fn validate_selected_web_search_credential_env(
    provider: &str,
    selected_env_name: &str,
) -> CliResult<String> {
    let trimmed = selected_env_name.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if let Some(normalized) = normalize_onboard_credential_env_name(trimmed) {
        return Ok(normalized);
    }

    let example_env_name = mvp::config::web_search_provider_descriptor(provider)
        .and_then(|descriptor| {
            descriptor
                .default_api_key_env
                .or_else(|| descriptor.api_key_env_names.first().copied())
        })
        .unwrap_or("WEB_SEARCH_API_KEY");

    Err(crate::access_terms::query_search_credential_source_validation_error(example_env_name))
}

fn apply_selected_web_search_credential(
    config: &mut mvp::config::LoongConfig,
    provider: &str,
    selection: WebSearchCredentialSelection,
) -> CliResult<()> {
    let next_value = match selection {
        WebSearchCredentialSelection::KeepCurrent => return Ok(()),
        WebSearchCredentialSelection::ClearConfigured => None,
        WebSearchCredentialSelection::UseEnv(env_name) => Some(format!("${{{}}}", env_name.trim())),
    };

    let updated = config
        .tools
        .web_search
        .set_configured_api_key_for_provider(provider, next_value);

    if !updated {
        return Err(format!(
            "unsupported web.search provider `{provider}`; credential update was skipped"
        ));
    }

    Ok(())
}

fn validate_selected_provider_credential_env(
    config: &mvp::config::LoongConfig,
    selected_env_name: &str,
) -> CliResult<String> {
    let trimmed = selected_env_name.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let mut candidate = config.clone();
    apply_selected_api_key_env(&mut candidate.provider, trimmed.to_owned());
    candidate.validate().map(|_| trimmed.to_owned())
}

fn non_interactive_preflight_warning_message(
    checks: &[OnboardCheck],
    options: &OnboardCommandOptions,
) -> String {
    let blocking_warning = checks.iter().find(|check| {
        check.level == OnboardCheckLevel::Warn
            && !is_explicitly_accepted_non_interactive_warning(check, options)
    });

    let detail = blocking_warning
        .map(|check| format!("{}: {}", check.name, check.detail))
        .unwrap_or_else(|| "unresolved warnings require interactive review".to_owned());

    format!(
        "onboard preflight failed: {detail}; rerun without --non-interactive to inspect and confirm them"
    )
}

pub fn preferred_api_key_env_default(config: &mvp::config::LoongConfig) -> String {
    provider_credential_policy::preferred_provider_credential_env_name(config)
}

pub fn collect_import_surfaces(config: &mvp::config::LoongConfig) -> Vec<ImportSurface> {
    crate::migration::collect_import_surfaces(config)
        .into_iter()
        .map(import_surface_from_migration)
        .collect()
}

pub fn collect_import_surfaces_with_channel_readiness(
    config: &mvp::config::LoongConfig,
    readiness: ChannelImportReadiness,
) -> Vec<ImportSurface> {
    crate::migration::collect_import_surfaces_with_channel_readiness(
        config,
        &to_migration_readiness(readiness),
    )
    .into_iter()
    .map(import_surface_from_migration)
    .collect()
}

fn secret_ref_has_inline_literal(secret_ref: Option<&SecretRef>) -> bool {
    let Some(secret_ref) = secret_ref else {
        return false;
    };

    secret_ref.inline_literal_value().is_some()
}

fn onboard_has_explicit_overrides(options: &OnboardCommandOptions) -> bool {
    option_has_non_empty_value(options.provider.as_deref())
        || option_has_non_empty_value(options.model.as_deref())
        || option_has_non_empty_value(options.api_key_env.as_deref())
        || option_has_non_empty_value(options.web_search_provider.as_deref())
        || option_has_non_empty_value(options.web_search_api_key_env.as_deref())
        || option_has_non_empty_value(options.personality.as_deref())
        || option_has_non_empty_value(options.memory_profile.as_deref())
        || option_has_non_empty_value(options.system_prompt.as_deref())
        || option_has_non_empty_value(env::var("LOONG_WEB_SEARCH_PROVIDER").ok().as_deref())
}

fn option_has_non_empty_value(raw: Option<&str>) -> bool {
    raw.is_some_and(|value| !value.trim().is_empty())
}

fn load_existing_output_config(output_path: &Path) -> Option<mvp::config::LoongConfig> {
    let path_str = output_path.to_str()?;
    mvp::config::load(Some(path_str))
        .ok()
        .map(|(_, config)| config)
}

pub fn should_skip_config_write(
    existing_config: Option<&mvp::config::LoongConfig>,
    draft: &mvp::config::LoongConfig,
) -> bool {
    existing_config.is_some_and(|existing| {
        if existing == draft {
            return true;
        }

        let existing_rendered = mvp::config::render(existing).ok();
        let draft_rendered = mvp::config::render(draft).ok();
        existing_rendered.is_some() && existing_rendered == draft_rendered
    })
}

pub fn parse_provider_kind(raw: &str) -> Option<mvp::config::ProviderKind> {
    mvp::config::ProviderKind::parse(raw)
}

pub fn parse_prompt_personality(raw: &str) -> Option<mvp::prompt::PromptPersonality> {
    mvp::prompt::parse_prompt_personality(raw)
}

pub fn parse_memory_profile(raw: &str) -> Option<mvp::config::MemoryProfile> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "window_only" | "window" => Some(mvp::config::MemoryProfile::WindowOnly),
        "window_plus_summary" | "summary" | "summary_window" => {
            Some(mvp::config::MemoryProfile::WindowPlusSummary)
        }
        "profile_plus_window" | "profile" | "profile_window" => {
            Some(mvp::config::MemoryProfile::ProfilePlusWindow)
        }
        _ => None,
    }
}

pub fn provider_default_api_key_env(kind: mvp::config::ProviderKind) -> Option<&'static str> {
    kind.default_api_key_env()
}

pub fn provider_kind_id(kind: mvp::config::ProviderKind) -> &'static str {
    kind.as_str()
}

pub fn provider_kind_display_name(kind: mvp::config::ProviderKind) -> &'static str {
    kind.display_name()
}

pub fn prompt_personality_id(personality: mvp::prompt::PromptPersonality) -> &'static str {
    personality.id()
}

pub fn memory_profile_id(profile: mvp::config::MemoryProfile) -> &'static str {
    match profile {
        mvp::config::MemoryProfile::WindowOnly => "window_only",
        mvp::config::MemoryProfile::WindowPlusSummary => "window_plus_summary",
        mvp::config::MemoryProfile::ProfilePlusWindow => "profile_plus_window",
    }
}

pub fn supported_provider_list() -> String {
    mvp::config::ProviderKind::all_sorted()
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn supported_personality_list() -> String {
    mvp::prompt::supported_prompt_personality_list()
}

pub fn supported_memory_profile_list() -> &'static str {
    "window_only, window_plus_summary, profile_plus_window"
}

fn resolve_write_plan(
    output_path: &Path,
    options: &OnboardCommandOptions,
    ui: &mut impl OnboardUi,
    context: &OnboardRuntimeContext,
) -> CliResult<ConfigWritePlan> {
    if !output_path.exists() {
        return Ok(ConfigWritePlan {
            force: false,
            backup_path: None,
        });
    }
    if options.force {
        return Ok(ConfigWritePlan {
            force: true,
            backup_path: None,
        });
    }

    if options.non_interactive {
        return Err(format!(
            "config {} already exists (use --force to overwrite)",
            output_path.display()
        ));
    }

    let existing_path = output_path.display().to_string();
    print_lines(
        ui,
        render_existing_config_write_header_lines_with_style(
            &existing_path,
            context.render_width,
            true,
        ),
    )?;
    let options = build_existing_config_write_screen_options();
    let selected = options
        .get(select_screen_option(
            ui,
            "Your choice",
            &options,
            Some("b"),
        )?)
        .ok_or_else(|| "existing-config write selection out of range".to_owned())?;
    match selected.key.as_str() {
        "o" => Ok(ConfigWritePlan {
            force: true,
            backup_path: None,
        }),
        "b" => Ok(ConfigWritePlan {
            force: true,
            backup_path: Some(resolve_backup_path(output_path)?),
        }),
        "c" => Err("onboarding cancelled: config file already exists".to_owned()),
        key => Err(format!(
            "unexpected existing-config write selection key: {key}"
        )),
    }
}

#[cfg(test)]
#[path = "onboard_cli_tests.rs"]
mod tests;
