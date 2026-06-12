use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::config::{AppConfig, SettingsSection};
use crate::ui::{components, theme};

pub struct SettingsWindow {
    config: AppConfig,
}

impl SettingsWindow {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(settings_navigation(self.config.active_settings_section))
                    .child(div().flex_1().p_6().child(settings_content(&self.config))),
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

fn settings_content(config: &AppConfig) -> AnyElement {
    match config.active_settings_section {
        SettingsSection::AiProviders => ai_provider_pane(config),
        section => settings_placeholder(section),
    }
}

fn ai_provider_pane(config: &AppConfig) -> AnyElement {
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
                .child(provider_editor(config)),
        )
        .child(
            div()
                .rounded(px(9.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface_tint())
                .p_3()
                .child(components::label(
                    "模型调用将在后续通过 Rig + 自研 Provider 抽象层接入。当前仅保存配置预览。",
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

fn provider_editor(config: &AppConfig) -> AnyElement {
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
                            let provider_id = provider.id.clone();
                            move || {
                                tracing::info!(component = "settings", provider_id = %provider_id, "provider test requested");
                            }
                        }))
                        .child(action_button("保存配置", provider.can_save(), {
                            let provider_id = provider.id.clone();
                            let can_save = provider.can_save();
                            move || {
                                if can_save {
                                    tracing::info!(component = "settings", provider_id = %provider_id, "provider save requested");
                                } else {
                                    tracing::warn!(component = "settings", provider_id = %provider_id, "save blocked until provider test matches current candidate");
                                }
                            }
                        })),
                )
                .child(components::label("预览按钮不会发起网络请求")),
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
        .child(components::label("此配置分区为预览入口。"))
        .into_any_element()
}
