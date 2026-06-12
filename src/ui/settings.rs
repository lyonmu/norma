use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::agent::provider::{ProviderCandidate, ProviderService};
use crate::config::{
    AppConfig, ConfigError, NormaConfig, ProviderApiType, ProviderConfigStatus, SettingsSection,
    write_config,
};
use crate::ui::{components, theme};

#[derive(Clone)]
struct SettingsWindowState {
    config: AppConfig,
    persisted_config: Option<NormaConfig>,
    config_file: Option<PathBuf>,
}

pub struct SettingsWindow {
    state: Arc<Mutex<SettingsWindowState>>,
}

impl SettingsWindow {
    pub fn new(
        config: AppConfig,
        persisted_config: Option<NormaConfig>,
        config_file: Option<PathBuf>,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(SettingsWindowState {
                config,
                persisted_config,
                config_file,
            })),
        }
    }
}

impl SettingsWindowState {
    fn test_selected_provider(
        &mut self,
        test_provider: impl Fn(
            ProviderCandidate<'_>,
        ) -> Result<
            crate::agent::provider::ProviderTestResult,
            crate::agent::provider::ProviderError,
        >,
    ) -> Result<(), String> {
        let Some(provider) = self.config.selected_provider() else {
            return Err("请选择一个提供商".to_string());
        };

        let selected_model = provider
            .models
            .iter()
            .find(|candidate| {
                candidate.model_id == provider.model || candidate.id == provider.model
            })
            .or_else(|| provider.default_model())
            .ok_or_else(|| "提供商缺少默认模型".to_string())?;

        let provider = crate::config::AiProviderConfig {
            id: provider.id.clone(),
            name: provider.name.clone(),
            api_type: match provider.protocol {
                crate::config::ProviderProtocol::OpenAi => ProviderApiType::OpenAi,
                crate::config::ProviderProtocol::Anthropic => ProviderApiType::Anthropic,
            },
            base_url: provider.base_url.clone(),
            api_key: provider.api_key_reference.clone(),
            is_default: provider.is_default,
            models: provider
                .models
                .iter()
                .map(|model| crate::config::AiModelConfig {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    model_id: model.model_id.clone(),
                    is_default: model.is_default,
                })
                .collect(),
        };
        let model = crate::config::AiModelConfig {
            id: selected_model.id.clone(),
            name: selected_model.name.clone(),
            model_id: selected_model.model_id.clone(),
            is_default: selected_model.is_default,
        };
        let candidate = ProviderCandidate {
            provider: &provider,
            model: &model,
        };

        match test_provider(candidate) {
            Ok(_) => {
                if let Some(provider) = self.config.selected_provider_mut() {
                    provider.status = ProviderConfigStatus::Complete;
                    provider.mark_tested();
                }
                Ok(())
            }
            Err(error) => {
                if let Some(provider) = self.config.selected_provider_mut() {
                    provider.status = ProviderConfigStatus::Invalid;
                    provider.tested_candidate_fingerprint = None;
                }
                Err(error.to_string())
            }
        }
    }

    fn save_selected_provider(
        &self,
        mut write: impl for<'a> FnMut(&Path, &'a NormaConfig) -> Result<(), ConfigError>,
    ) -> Result<(), String> {
        let Some(provider) = self.config.selected_provider() else {
            return Err("请选择一个提供商".to_string());
        };
        if !provider.can_save() {
            return Err("保存被阻止：请先测试当前候选并保持配置一致".to_string());
        }

        let Some(path) = self.config_file.as_deref() else {
            return Err("缺少配置文件路径".to_string());
        };
        let Some(config) = self.persisted_config.as_ref() else {
            return Err("缺少可写入的配置".to_string());
        };

        write(path, config).map_err(|error| error.to_string())
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let config = self.state.lock().unwrap().config.clone();
        div()
            .size_full()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .flex()
            .flex_col()
            .child(settings_header())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(settings_navigation(config.active_settings_section))
                    .child(
                        div()
                            .flex_1()
                            .p_6()
                            .child(settings_content(&self.state, &config)),
                    ),
            )
    }
}

fn settings_header() -> AnyElement {
    div()
        .h(px(56.))
        .px_5()
        .border_b_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(16.))
                .child("设置"),
        )
        .child(components::pill("应用级配置", false))
        .into_any_element()
}

fn settings_navigation(active: SettingsSection) -> AnyElement {
    div()
        .w(px(240.))
        .h_full()
        .bg(theme::surface())
        .border_r_1()
        .border_color(theme::border())
        .p_4()
        .flex()
        .flex_col()
        .gap_1()
        .children(SettingsSection::ALL.into_iter().map(|section| {
            let selected = section == active;
            div()
                .rounded(px(8.))
                .px_3()
                .py_2()
                .bg(if selected {
                    theme::surface_tint()
                } else {
                    theme::surface()
                })
                .text_color(if selected {
                    theme::text()
                } else {
                    theme::muted()
                })
                .font_weight(if selected {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .child(section.label())
        }))
        .into_any_element()
}

fn settings_content(state: &Arc<Mutex<SettingsWindowState>>, config: &AppConfig) -> AnyElement {
    match config.active_settings_section {
        SettingsSection::AiProviders => ai_provider_pane(state, config),
        section => settings_placeholder(section),
    }
}

fn ai_provider_pane(state: &Arc<Mutex<SettingsWindowState>>, config: &AppConfig) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_5()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("AI 提供商"),
                        )
                        .child(components::label(
                            "测试通过后才能保存配置。配置会写入本机 ~/.norma/config.toml。",
                        )),
                )
                .child(components::pill("+ 新增提供商", true)),
        )
        .child(
            div()
                .flex()
                .gap_5()
                .child(provider_list(config))
                .child(provider_editor(state, config)),
        )
        .child(
            div()
                .rounded(px(9.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface_tint())
                .p_3()
                .child(components::label(
                    "测试连接会调用当前候选提供商；保存仅在最近一次测试仍匹配当前候选时可用。",
                )),
        )
        .into_any_element()
}

fn provider_list(config: &AppConfig) -> AnyElement {
    div()
        .w(px(360.))
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(theme::border())
                .child(components::section_title("提供商")),
        )
        .children(config.providers.iter().map(|provider| {
            let selected = config.selected_provider_id.as_deref() == Some(provider.id.as_str());
            div()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(theme::border())
                .bg(if selected {
                    theme::surface_tint()
                } else {
                    theme::surface()
                })
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(provider.name.clone()),
                                )
                                .child(if provider.is_default {
                                    components::pill("默认", true)
                                } else {
                                    components::pill("候选", false)
                                }),
                        )
                        .child(components::pill(provider.protocol.label(), false)),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(components::label(provider.model.clone()))
                        .child(status_label(provider.status)),
                )
                .child(model_list(config, provider.id.as_str()))
        }))
        .into_any_element()
}

fn status_label(status: crate::config::ProviderConfigStatus) -> AnyElement {
    let color = match status {
        crate::config::ProviderConfigStatus::Complete => theme::green(),
        crate::config::ProviderConfigStatus::Invalid => theme::red(),
        crate::config::ProviderConfigStatus::Incomplete
        | crate::config::ProviderConfigStatus::PreviewUnvalidated => theme::muted(),
    };
    div()
        .text_size(px(13.))
        .text_color(color)
        .child(status.label())
        .into_any_element()
}

fn provider_editor(state: &Arc<Mutex<SettingsWindowState>>, config: &AppConfig) -> AnyElement {
    let Some(provider) = config.selected_provider() else {
        return div()
            .flex_1()
            .rounded(px(10.))
            .border_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .p_5()
            .child(components::label("选择一个提供商进行配置。"))
            .into_any_element();
    };

    div()
        .flex_1()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_5()
        .flex()
        .flex_col()
        .gap_4()
        .child(components::section_title("提供商配置"))
        .child(form_row("名称", provider.name.clone()))
        .child(protocol_segment(provider.protocol))
        .child(form_row("Base URL", provider.base_url.clone()))
        .child(form_row("API Key", provider.masked_api_key()))
        .child(form_row("模型", provider.model.clone()))
        .child(model_list(config, provider.id.as_str()))
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(action_button("测试连接", true, {
                            let state = Arc::clone(state);
                            let provider_id = provider.id.clone();
                            move || {
                                let mut guard = state.lock().unwrap();
                                match guard.test_selected_provider(ProviderService::test_provider) {
                                    Ok(()) => {
                                        if let Some(provider) = guard.config.selected_provider() {
                                            tracing::info!(component = "settings", provider_id = %provider_id, model_id = %provider.model, "provider test completed");
                                        }
                                    }
                                    Err(error) => {
                                        tracing::warn!(component = "settings", provider_id = %provider_id, error = %error, "provider test failed");
                                    }
                                }
                            }
                        }))
                        .child(action_button("保存配置", provider.can_save(), {
                            let state = Arc::clone(state);
                            let provider_id = provider.id.clone();
                            move || {
                                let guard = state.lock().unwrap();
                                match guard.save_selected_provider(|path, config| write_config(path, config)) {
                                    Ok(()) => {
                                        tracing::info!(component = "settings", provider_id = %provider_id, "provider config saved");
                                    }
                                    Err(error) => {
                                        tracing::warn!(component = "settings", provider_id = %provider_id, error = %error, "provider save blocked");
                                    }
                                }
                            }
                        })),
                )
                .child(components::label("测试连接会调用当前候选提供商，保存仅在测试匹配时启用")),
        )
        .into_any_element()
}

fn model_list(config: &AppConfig, provider_id: &str) -> AnyElement {
    let Some(provider) = config
        .providers
        .iter()
        .find(|candidate| candidate.id == provider_id)
    else {
        return div().into_any_element();
    };

    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(components::label("模型列表"))
        .children(provider.models.iter().map(|model| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(model.name.clone())
                        .child(components::label(model.model_id.clone())),
                )
                .child(if model.is_default {
                    components::pill("默认模型", true)
                } else {
                    components::pill("候选", false)
                })
        }))
        .into_any_element()
}

fn action_button(label: &'static str, active: bool, on_click: impl Fn() + 'static) -> AnyElement {
    div()
        .id(SharedString::from(format!("settings-{label}")))
        .child(components::pill(label, active))
        .on_click(move |_, _, _| on_click())
        .into_any_element()
}

fn form_row(label: &str, value: impl Into<String>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::label(label))
        .child(
            div()
                .h(px(38.))
                .rounded(px(8.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface())
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .child(value.into()),
        )
        .into_any_element()
}

fn protocol_segment(active: crate::config::ProviderProtocol) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::label("协议类型"))
        .child(
            div()
                .rounded(px(8.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::app_bg())
                .p_1()
                .flex()
                .gap_1()
                .children(
                    crate::config::ProviderProtocol::ALL
                        .into_iter()
                        .map(|protocol| {
                            let selected = protocol == active;
                            div()
                                .flex_1()
                                .rounded(px(6.))
                                .px_3()
                                .py_2()
                                .bg(if selected {
                                    theme::surface()
                                } else {
                                    theme::app_bg()
                                })
                                .text_color(if selected {
                                    theme::text()
                                } else {
                                    theme::muted()
                                })
                                .font_weight(if selected {
                                    gpui::FontWeight::SEMIBOLD
                                } else {
                                    gpui::FontWeight::NORMAL
                                })
                                .child(protocol.label())
                        }),
                ),
        )
        .into_any_element()
}

fn settings_placeholder(section: SettingsSection) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_5()
        .child(components::section_title(section.label()))
        .child(components::label("此配置分区暂未开放。"))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_runtime_config() -> NormaConfig {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut config = NormaConfig::default_for(&paths);
        config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI 默认".to_string(),
            api_type: ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-openai-default".to_string(),
            is_default: true,
            models: vec![crate::config::AiModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o mini".to_string(),
                model_id: "gpt-4o-mini".to_string(),
                is_default: true,
            }],
        }];
        config
    }

    #[test]
    fn test_action_marks_selected_provider_as_tested() {
        let runtime_config = sample_runtime_config();
        let config = AppConfig::from_norma_config(&runtime_config);
        let mut state = SettingsWindowState {
            config,
            persisted_config: Some(runtime_config),
            config_file: None,
        };

        state
            .test_selected_provider(|candidate| {
                assert_eq!(candidate.provider.id, "openai-default");
                assert_eq!(candidate.model.model_id, "gpt-4o-mini");
                Ok(crate::agent::provider::ProviderTestResult {
                    provider_id: candidate.provider.id.clone(),
                    model_id: candidate.model.model_id.clone(),
                    content_preview: "ok".to_string(),
                })
            })
            .unwrap();

        let provider = state.config.selected_provider().unwrap();
        assert!(provider.tested_candidate_fingerprint.is_some());
        assert!(provider.can_save());
        assert_eq!(provider.status, ProviderConfigStatus::Complete);
    }

    #[test]
    fn save_action_writes_current_runtime_config_when_test_is_fresh() {
        let runtime_config = sample_runtime_config();
        let mut config = AppConfig::from_norma_config(&runtime_config);
        config.selected_provider_mut().unwrap().mark_tested();
        let state = SettingsWindowState {
            config,
            persisted_config: Some(runtime_config.clone()),
            config_file: Some(std::env::temp_dir().join("norma-settings-test.toml")),
        };

        let mut saved_path = None;
        let mut saved_config = None;
        state
            .save_selected_provider(|path, config| {
                saved_path = Some(path.to_path_buf());
                saved_config = Some(config.clone());
                Ok(())
            })
            .unwrap();

        assert_eq!(saved_path, state.config_file.clone());
        assert_eq!(saved_config.unwrap(), runtime_config);
    }

    #[test]
    fn save_action_blocks_when_candidate_is_stale() {
        let runtime_config = sample_runtime_config();
        let config = AppConfig::from_norma_config(&runtime_config);
        let state = SettingsWindowState {
            config,
            persisted_config: Some(runtime_config),
            config_file: Some(std::env::temp_dir().join("norma-settings-test.toml")),
        };

        let err = state
            .save_selected_provider(|_, _| -> Result<(), ConfigError> { Ok(()) })
            .unwrap_err();

        assert!(err.contains("测试当前候选"));
    }
}
