use loong_contracts::SecretRef;

use super::{
    ConfigValidationDiagnostic, DingtalkChannelConfig, DiscordChannelConfig, EmailChannelConfig,
    FeishuChannelConfig, GoogleChatChannelConfig, ImessageChannelConfig, LineChannelConfig,
    LoongConfig, MatrixChannelConfig, MattermostChannelConfig, NextcloudTalkChannelConfig,
    SlackChannelConfig, SynologyChatChannelConfig, TeamsChannelConfig, TelegramChannelConfig,
    WebhookChannelConfig, WecomChannelConfig, WhatsappChannelConfig,
};

const PROVIDER_SECRET_HEADER_NAMES: &[&str] = &["authorization", "x-api-key"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretReferenceKind {
    Env,
    File,
    Exec,
    InlineLiteral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretObservation {
    pub field_path: String,
    pub kind: SecretReferenceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SecretObservationCounts {
    pub env: usize,
    pub file: usize,
    pub exec: usize,
    pub inline_literal: usize,
}

impl SecretObservationCounts {
    fn record(&mut self, kind: SecretReferenceKind) {
        match kind {
            SecretReferenceKind::Env => self.env += 1,
            SecretReferenceKind::File => self.file += 1,
            SecretReferenceKind::Exec => self.exec += 1,
            SecretReferenceKind::InlineLiteral => self.inline_literal += 1,
        }
    }
}

pub fn collect_env_pointer_diagnostics(config: &LoongConfig) -> Vec<ConfigValidationDiagnostic> {
    config
        .validation_diagnostics()
        .into_iter()
        .filter(|diagnostic| diagnostic.code.starts_with("config.env_pointer."))
        .collect()
}

pub fn collect_secret_observations(config: &LoongConfig) -> Vec<SecretObservation> {
    let mut observations = Vec::new();

    collect_provider_secret_observations(config, &mut observations);
    collect_web_search_secret_observations(config, &mut observations);
    collect_channel_secret_observations(config, &mut observations);

    observations
}

pub fn summarize_secret_observations(
    observations: &[SecretObservation],
) -> SecretObservationCounts {
    let mut counts = SecretObservationCounts::default();

    for observation in observations {
        counts.record(observation.kind);
    }

    counts
}

pub fn observation_paths_for_kind(
    observations: &[SecretObservation],
    kind: SecretReferenceKind,
) -> Vec<String> {
    observations
        .iter()
        .filter(|observation| observation.kind == kind)
        .map(|observation| observation.field_path.clone())
        .collect()
}

fn collect_provider_secret_observations(
    config: &LoongConfig,
    observations: &mut Vec<SecretObservation>,
) {
    collect_single_provider_secret_observations("provider", &config.provider, observations);

    for (profile_id, profile) in &config.providers {
        let field_prefix = format!("providers.{profile_id}");
        collect_single_provider_secret_observations(
            field_prefix.as_str(),
            &profile.provider,
            observations,
        );
    }
}

fn collect_single_provider_secret_observations(
    field_prefix: &str,
    provider: &super::ProviderConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let api_key_path = format!("{field_prefix}.api_key");
    push_secret_ref_observation(observations, api_key_path, provider.api_key.as_ref());

    let oauth_path = format!("{field_prefix}.oauth_access_token");
    push_secret_ref_observation(
        observations,
        oauth_path,
        provider.oauth_access_token.as_ref(),
    );

    collect_provider_header_secret_observations(field_prefix, provider, observations);
}

fn collect_provider_header_secret_observations(
    field_prefix: &str,
    provider: &super::ProviderConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (header_name, header_value) in &provider.headers {
        if !provider_header_may_contain_secret(header_name.as_str()) {
            continue;
        }

        let field_path = format!("{field_prefix}.headers.{header_name}");
        push_string_secret_observation(observations, field_path, Some(header_value.as_str()));
    }
}

fn provider_header_may_contain_secret(header_name: &str) -> bool {
    PROVIDER_SECRET_HEADER_NAMES
        .iter()
        .any(|candidate| header_name.eq_ignore_ascii_case(candidate))
}

fn collect_web_search_secret_observations(
    config: &LoongConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, value) in [
        (
            "tools.web_search.brave_api_key",
            config.tools.web_search.brave_api_key.as_deref(),
        ),
        (
            "tools.web_search.tavily_api_key",
            config.tools.web_search.tavily_api_key.as_deref(),
        ),
        (
            "tools.web_search.perplexity_api_key",
            config.tools.web_search.perplexity_api_key.as_deref(),
        ),
        (
            "tools.web_search.exa_api_key",
            config.tools.web_search.exa_api_key.as_deref(),
        ),
        (
            "tools.web_search.firecrawl_api_key",
            config.tools.web_search.firecrawl_api_key.as_deref(),
        ),
        (
            "tools.web_search.jina_api_key",
            config.tools.web_search.jina_api_key.as_deref(),
        ),
    ] {
        push_string_secret_observation(observations, field_path.to_owned(), value);
    }
}

fn collect_channel_secret_observations(
    config: &LoongConfig,
    observations: &mut Vec<SecretObservation>,
) {
    collect_telegram_secret_observations(&config.telegram, observations);
    collect_feishu_secret_observations(&config.feishu, observations);
    collect_matrix_secret_observations(&config.matrix, observations);
    collect_wecom_secret_observations(&config.wecom, observations);
    collect_discord_secret_observations(&config.discord, observations);
    collect_line_secret_observations(&config.line, observations);
    collect_dingtalk_secret_observations(&config.dingtalk, observations);
    collect_webhook_secret_observations(&config.webhook, observations);
    collect_email_secret_observations(&config.email, observations);
    collect_slack_secret_observations(&config.slack, observations);
    collect_google_chat_secret_observations(&config.google_chat, observations);
    collect_mattermost_secret_observations(&config.mattermost, observations);
    collect_nextcloud_talk_secret_observations(&config.nextcloud_talk, observations);
    collect_synology_chat_secret_observations(&config.synology_chat, observations);
    collect_teams_secret_observations(&config.teams, observations);
    collect_imessage_secret_observations(&config.imessage, observations);
    collect_whatsapp_secret_observations(&config.whatsapp, observations);
}

fn collect_telegram_secret_observations(
    config: &TelegramChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let bot_token_path = "telegram.bot_token".to_owned();
    push_secret_ref_observation(observations, bot_token_path, config.bot_token.as_ref());

    for (account_id, account) in &config.accounts {
        let bot_token_path = format!("telegram.accounts.{account_id}.bot_token");
        push_secret_ref_observation(observations, bot_token_path, account.bot_token.as_ref());
    }
}

fn collect_feishu_secret_observations(
    config: &FeishuChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("feishu.app_id".to_owned(), config.app_id.as_ref()),
        ("feishu.app_secret".to_owned(), config.app_secret.as_ref()),
        (
            "feishu.verification_token".to_owned(),
            config.verification_token.as_ref(),
        ),
        ("feishu.encrypt_key".to_owned(), config.encrypt_key.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("app_id", account.app_id.as_ref()),
            ("app_secret", account.app_secret.as_ref()),
            ("verification_token", account.verification_token.as_ref()),
            ("encrypt_key", account.encrypt_key.as_ref()),
        ] {
            let field_path = format!("feishu.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_matrix_secret_observations(
    config: &MatrixChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let access_token_path = "matrix.access_token".to_owned();
    push_secret_ref_observation(
        observations,
        access_token_path,
        config.access_token.as_ref(),
    );

    for (account_id, account) in &config.accounts {
        let access_token_path = format!("matrix.accounts.{account_id}.access_token");
        push_secret_ref_observation(
            observations,
            access_token_path,
            account.access_token.as_ref(),
        );
    }
}

fn collect_wecom_secret_observations(
    config: &WecomChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("wecom.bot_id".to_owned(), config.bot_id.as_ref()),
        ("wecom.secret".to_owned(), config.secret.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("bot_id", account.bot_id.as_ref()),
            ("secret", account.secret.as_ref()),
        ] {
            let field_path = format!("wecom.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_discord_secret_observations(
    config: &DiscordChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let bot_token_path = "discord.bot_token".to_owned();
    push_secret_ref_observation(observations, bot_token_path, config.bot_token.as_ref());

    for (account_id, account) in &config.accounts {
        let bot_token_path = format!("discord.accounts.{account_id}.bot_token");
        push_secret_ref_observation(observations, bot_token_path, account.bot_token.as_ref());
    }
}

fn collect_line_secret_observations(
    config: &LineChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        (
            "line.channel_access_token".to_owned(),
            config.channel_access_token.as_ref(),
        ),
        ("line.channel_secret".to_owned(), config.channel_secret.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("channel_access_token", account.channel_access_token.as_ref()),
            ("channel_secret", account.channel_secret.as_ref()),
        ] {
            let field_path = format!("line.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_dingtalk_secret_observations(
    config: &DingtalkChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("dingtalk.webhook_url".to_owned(), config.webhook_url.as_ref()),
        ("dingtalk.secret".to_owned(), config.secret.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("webhook_url", account.webhook_url.as_ref()),
            ("secret", account.secret.as_ref()),
        ] {
            let field_path = format!("dingtalk.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_webhook_secret_observations(
    config: &WebhookChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("webhook.endpoint_url".to_owned(), config.endpoint_url.as_ref()),
        ("webhook.auth_token".to_owned(), config.auth_token.as_ref()),
        (
            "webhook.signing_secret".to_owned(),
            config.signing_secret.as_ref(),
        ),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("endpoint_url", account.endpoint_url.as_ref()),
            ("auth_token", account.auth_token.as_ref()),
            ("signing_secret", account.signing_secret.as_ref()),
        ] {
            let field_path = format!("webhook.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_email_secret_observations(
    config: &EmailChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("email.smtp_username".to_owned(), config.smtp_username.as_ref()),
        ("email.smtp_password".to_owned(), config.smtp_password.as_ref()),
        ("email.imap_username".to_owned(), config.imap_username.as_ref()),
        ("email.imap_password".to_owned(), config.imap_password.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("smtp_username", account.smtp_username.as_ref()),
            ("smtp_password", account.smtp_password.as_ref()),
            ("imap_username", account.imap_username.as_ref()),
            ("imap_password", account.imap_password.as_ref()),
        ] {
            let field_path = format!("email.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_slack_secret_observations(
    config: &SlackChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let bot_token_path = "slack.bot_token".to_owned();
    push_secret_ref_observation(observations, bot_token_path, config.bot_token.as_ref());

    for (account_id, account) in &config.accounts {
        let bot_token_path = format!("slack.accounts.{account_id}.bot_token");
        push_secret_ref_observation(observations, bot_token_path, account.bot_token.as_ref());
    }
}

fn collect_google_chat_secret_observations(
    config: &GoogleChatChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let webhook_path = "google_chat.webhook_url".to_owned();
    push_secret_ref_observation(observations, webhook_path, config.webhook_url.as_ref());

    for (account_id, account) in &config.accounts {
        let webhook_path = format!("google_chat.accounts.{account_id}.webhook_url");
        push_secret_ref_observation(observations, webhook_path, account.webhook_url.as_ref());
    }
}

fn collect_mattermost_secret_observations(
    config: &MattermostChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let bot_token_path = "mattermost.bot_token".to_owned();
    push_secret_ref_observation(observations, bot_token_path, config.bot_token.as_ref());

    for (account_id, account) in &config.accounts {
        let bot_token_path = format!("mattermost.accounts.{account_id}.bot_token");
        push_secret_ref_observation(observations, bot_token_path, account.bot_token.as_ref());
    }
}

fn collect_nextcloud_talk_secret_observations(
    config: &NextcloudTalkChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let shared_secret_path = "nextcloud_talk.shared_secret".to_owned();
    push_secret_ref_observation(
        observations,
        shared_secret_path,
        config.shared_secret.as_ref(),
    );

    for (account_id, account) in &config.accounts {
        let shared_secret_path = format!("nextcloud_talk.accounts.{account_id}.shared_secret");
        push_secret_ref_observation(
            observations,
            shared_secret_path,
            account.shared_secret.as_ref(),
        );
    }
}

fn collect_synology_chat_secret_observations(
    config: &SynologyChatChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("synology_chat.token".to_owned(), config.token.as_ref()),
        (
            "synology_chat.incoming_url".to_owned(),
            config.incoming_url.as_ref(),
        ),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("token", account.token.as_ref()),
            ("incoming_url", account.incoming_url.as_ref()),
        ] {
            let field_path = format!("synology_chat.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_teams_secret_observations(
    config: &TeamsChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("teams.webhook_url".to_owned(), config.webhook_url.as_ref()),
        ("teams.app_id".to_owned(), config.app_id.as_ref()),
        ("teams.app_password".to_owned(), config.app_password.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("webhook_url", account.webhook_url.as_ref()),
            ("app_id", account.app_id.as_ref()),
            ("app_password", account.app_password.as_ref()),
        ] {
            let field_path = format!("teams.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn collect_imessage_secret_observations(
    config: &ImessageChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    let bridge_token_path = "imessage.bridge_token".to_owned();
    push_secret_ref_observation(
        observations,
        bridge_token_path,
        config.bridge_token.as_ref(),
    );

    for (account_id, account) in &config.accounts {
        let bridge_token_path = format!("imessage.accounts.{account_id}.bridge_token");
        push_secret_ref_observation(
            observations,
            bridge_token_path,
            account.bridge_token.as_ref(),
        );
    }
}

fn collect_whatsapp_secret_observations(
    config: &WhatsappChannelConfig,
    observations: &mut Vec<SecretObservation>,
) {
    for (field_path, secret_ref) in [
        ("whatsapp.access_token".to_owned(), config.access_token.as_ref()),
        ("whatsapp.verify_token".to_owned(), config.verify_token.as_ref()),
        ("whatsapp.app_secret".to_owned(), config.app_secret.as_ref()),
    ] {
        push_secret_ref_observation(observations, field_path, secret_ref);
    }

    for (account_id, account) in &config.accounts {
        for (suffix, secret_ref) in [
            ("access_token", account.access_token.as_ref()),
            ("verify_token", account.verify_token.as_ref()),
            ("app_secret", account.app_secret.as_ref()),
        ] {
            let field_path = format!("whatsapp.accounts.{account_id}.{suffix}");
            push_secret_ref_observation(observations, field_path, secret_ref);
        }
    }
}

fn push_string_secret_observation(
    observations: &mut Vec<SecretObservation>,
    field_path: String,
    raw_value: Option<&str>,
) {
    let Some(raw_value) = raw_value else {
        return;
    };

    let trimmed_value = raw_value.trim();
    if trimmed_value.is_empty() {
        return;
    }

    let secret_ref = SecretRef::Inline(trimmed_value.to_owned());
    let Some(kind) = classify_secret_ref_kind(&secret_ref) else {
        return;
    };

    observations.push(SecretObservation { field_path, kind });
}

fn push_secret_ref_observation(
    observations: &mut Vec<SecretObservation>,
    field_path: String,
    secret_ref: Option<&SecretRef>,
) {
    let Some(secret_ref) = secret_ref else {
        return;
    };

    let Some(kind) = classify_secret_ref_kind(secret_ref) else {
        return;
    };

    observations.push(SecretObservation { field_path, kind });
}

fn classify_secret_ref_kind(secret_ref: &SecretRef) -> Option<SecretReferenceKind> {
    match secret_ref {
        SecretRef::Env { .. } => Some(SecretReferenceKind::Env),
        SecretRef::File { .. } => Some(SecretReferenceKind::File),
        SecretRef::Exec { .. } => Some(SecretReferenceKind::Exec),
        SecretRef::Inline(_) => {
            if secret_ref.inline_literal_value().is_some() {
                return Some(SecretReferenceKind::InlineLiteral);
            }

            if secret_ref.explicit_env_name().is_some() {
                return Some(SecretReferenceKind::Env);
            }

            None
        }
    }
}
