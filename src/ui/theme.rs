use gpui::{Hsla, hsla, px};

pub const TOOLBAR_HEIGHT: gpui::Pixels = px(56.);
pub const SIDEBAR_WIDTH: gpui::Pixels = px(320.);
pub const INSPECTOR_WIDTH: gpui::Pixels = px(410.);

pub fn app_bg() -> Hsla {
    hsla(220. / 360., 0.16, 0.97, 1.)
}

pub fn surface() -> Hsla {
    hsla(0., 0., 1., 1.)
}

pub fn surface_tint() -> Hsla {
    hsla(218. / 360., 0.35, 0.96, 1.)
}

pub fn border() -> Hsla {
    hsla(220. / 360., 0.16, 0.88, 1.)
}

pub fn text() -> Hsla {
    hsla(222. / 360., 0.25, 0.13, 1.)
}

pub fn muted() -> Hsla {
    hsla(220. / 360., 0.08, 0.46, 1.)
}

pub fn blue() -> Hsla {
    hsla(218. / 360., 0.88, 0.56, 1.)
}

pub fn green() -> Hsla {
    hsla(145. / 360., 0.54, 0.40, 1.)
}

pub fn red() -> Hsla {
    hsla(356. / 360., 0.71, 0.52, 1.)
}
