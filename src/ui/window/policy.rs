use gpui::{Bounds, Pixels, Size, WindowBounds, WindowOptions, point, px, size};

use crate::config::WindowConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowSizeClass {
    Compact,
    Medium,
    Wide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSizeClass {
    Stacked,
    Inline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkbenchLayout {
    pub size_class: WindowSizeClass,
    pub sidebar_inline: bool,
    pub inspector_inline: bool,
    pub show_status_pills: bool,
}

impl WorkbenchLayout {
    pub fn for_width(width: Pixels) -> Self {
        if width < px(1120.) {
            Self {
                size_class: WindowSizeClass::Compact,
                sidebar_inline: false,
                inspector_inline: false,
                show_status_pills: false,
            }
        } else if width < px(1280.) {
            Self {
                size_class: WindowSizeClass::Medium,
                sidebar_inline: true,
                inspector_inline: false,
                show_status_pills: false,
            }
        } else {
            Self {
                size_class: WindowSizeClass::Wide,
                sidebar_inline: true,
                inspector_inline: true,
                show_status_pills: true,
            }
        }
    }
}

pub struct WindowPolicy;

impl WindowPolicy {
    pub fn main_content_size(config: Option<&WindowConfig>) -> Size<Pixels> {
        let (width, height) = config
            .map(|config| (config.width as f32, config.height as f32))
            .unwrap_or((1440., 1024.));

        size(px(width.max(1024.)), px(height.max(700.)))
    }

    pub fn main_window_options(config: Option<&WindowConfig>) -> WindowOptions {
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(80.), px(80.)),
                Self::main_content_size(config),
            ))),
            window_min_size: Some(size(px(1024.), px(700.))),
            ..WindowOptions::default()
        }
    }

    pub fn settings_window_options() -> WindowOptions {
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(180.), px(120.)),
                size(px(960.), px(720.)),
            ))),
            window_min_size: Some(size(px(840.), px(620.))),
            ..WindowOptions::default()
        }
    }

    pub fn settings_size_class(width: Pixels) -> SettingsSizeClass {
        if width < px(920.) {
            SettingsSizeClass::Stacked
        } else {
            SettingsSizeClass::Inline
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, Pixels, WindowBounds, WindowOptions, px, size};

    use super::{SettingsSizeClass, WindowPolicy, WindowSizeClass, WorkbenchLayout};
    use crate::config::WindowConfig;

    fn windowed_bounds(options: &WindowOptions) -> Bounds<Pixels> {
        match options.window_bounds {
            Some(WindowBounds::Windowed(bounds)) => bounds,
            bounds => panic!("expected windowed bounds, got {bounds:?}"),
        }
    }

    #[test]
    fn main_content_size_clamps_each_dimension_to_the_minimum() {
        let both_below_minimum = WindowConfig {
            width: 900,
            height: 680,
        };
        let width_below_minimum = WindowConfig {
            width: 900,
            height: 900,
        };
        let height_below_minimum = WindowConfig {
            width: 1200,
            height: 680,
        };

        assert_eq!(
            WindowPolicy::main_content_size(Some(&both_below_minimum)),
            size(px(1024.), px(700.))
        );
        assert_eq!(
            WindowPolicy::main_content_size(Some(&width_below_minimum)),
            size(px(1024.), px(900.))
        );
        assert_eq!(
            WindowPolicy::main_content_size(Some(&height_below_minimum)),
            size(px(1200.), px(700.))
        );
    }

    #[test]
    fn main_content_size_preserves_configured_dimensions_above_the_minimum() {
        let config = WindowConfig {
            width: 1600,
            height: 1000,
        };

        assert_eq!(
            WindowPolicy::main_content_size(Some(&config)),
            size(px(1600.), px(1000.))
        );
    }

    #[test]
    fn workbench_size_class_uses_expected_boundaries() {
        assert_eq!(
            WorkbenchLayout::for_width(px(1024.)).size_class,
            WindowSizeClass::Compact
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1119.)).size_class,
            WindowSizeClass::Compact
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1120.)).size_class,
            WindowSizeClass::Medium
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1279.)).size_class,
            WindowSizeClass::Medium
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1280.)).size_class,
            WindowSizeClass::Wide
        );
    }

    #[test]
    fn workbench_layout_projects_each_size_class() {
        assert_eq!(
            WorkbenchLayout::for_width(px(1024.)),
            WorkbenchLayout {
                size_class: WindowSizeClass::Compact,
                sidebar_inline: false,
                inspector_inline: false,
                show_status_pills: false,
            }
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1120.)),
            WorkbenchLayout {
                size_class: WindowSizeClass::Medium,
                sidebar_inline: true,
                inspector_inline: false,
                show_status_pills: false,
            }
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1280.)),
            WorkbenchLayout {
                size_class: WindowSizeClass::Wide,
                sidebar_inline: true,
                inspector_inline: true,
                show_status_pills: true,
            }
        );
    }

    #[test]
    fn settings_size_class_switches_to_inline_at_920_pixels() {
        assert_eq!(
            WindowPolicy::settings_size_class(px(919.)),
            SettingsSizeClass::Stacked
        );
        assert_eq!(
            WindowPolicy::settings_size_class(px(920.)),
            SettingsSizeClass::Inline
        );
    }

    #[test]
    fn main_window_options_define_bounds_and_minimum_size() {
        let options = WindowPolicy::main_window_options(None);
        let bounds = windowed_bounds(&options);

        assert_eq!(bounds.origin.x, px(80.));
        assert_eq!(bounds.origin.y, px(80.));
        assert_eq!(bounds.size, size(px(1440.), px(1024.)));
        assert_eq!(options.window_min_size, Some(size(px(1024.), px(700.))));
        assert!(options.is_resizable);
    }

    #[test]
    fn settings_window_options_define_bounds_and_minimum_size() {
        let options = WindowPolicy::settings_window_options();
        let bounds = windowed_bounds(&options);

        assert_eq!(bounds.origin.x, px(180.));
        assert_eq!(bounds.origin.y, px(120.));
        assert_eq!(bounds.size, size(px(960.), px(720.)));
        assert_eq!(options.window_min_size, Some(size(px(840.), px(620.))));
        assert!(options.is_resizable);
    }
}
