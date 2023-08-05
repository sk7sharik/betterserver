use fltk::{app::{self, App}, button::Button, frame::Frame, prelude::*, window::Window, group::{Tabs, Pack, PackType, Group}};
use fltk_theme::{ColorTheme, color_themes, WidgetTheme, ThemeType, WidgetScheme, SchemeType, widget_themes};

pub(crate) struct GUI
{

}

impl GUI
{
    pub fn new() -> GUI {
        GUI {}
    }

    pub fn run(&mut self) {
        let app = app::App::default();
        let widget_theme = WidgetTheme::new(ThemeType::Dark);
        widget_theme.apply();

        let mut win = Window::new(100, 100, 840, 480, "BetterServer");
        {
            let mut tabs = Tabs::new(0, 0, win.w(), win.h(), "");
            {
                let mut general = Group::new(0, 30, win.w(), win.h() - 30, "General");
                general.end();
                let mut general = Group::new(0, 30, win.w(), win.h() - 30, "General 2");
                general.end();
                let mut general = Group::new(0, 30, win.w(), win.h() - 30, "General 3");
                general.end();
            }
            tabs.end();
        }
        win.end();

        win.show();
        app.run().unwrap();
    }
}