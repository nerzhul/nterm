use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button, HeaderBar, Label, Notebook};
use vte4::Terminal;
use vte4::prelude::*;

use crate::config::{Config, KeyBindings};
use crate::strings as s;

use super::pane::count_leaves;
use super::regexes::MatchRegexes;
use super::tabs::{build_initial_pane, TabState};

pub struct AppCtx {
    pub app: Application,
    pub window: ApplicationWindow,
    pub config: Rc<Config>,
    pub bindings: KeyBindings,
    pub regexes: Rc<MatchRegexes>,
    pub notebook: Notebook,
    pub tabs: Rc<RefCell<Vec<TabState>>>,
    pub focused: Rc<RefCell<Option<Terminal>>>,
    pub search_button: Button,
    pub split_h_button: Button,
    pub split_v_button: Button,
    pub close_pane_button: Button,
    pub refresh_close_pane_button: Rc<dyn Fn()>,
}

impl AppCtx {
    pub fn new(app: &Application) -> Self {
        let config = Rc::new(Config::load());
        let bindings = config.get_bindings();
        let regexes = Rc::new(MatchRegexes::compile());

        let notebook = Notebook::builder()
            .scrollable(true)
            .show_tabs(false)
            .build();
        notebook.set_tab_pos(gtk4::PositionType::Top);

        let header_bar = HeaderBar::new();
        header_bar.set_show_title_buttons(true);
        header_bar.set_title_widget(Some(&Label::new(Some(s::APP_TITLE))));

        let search_button = Button::builder()
            .icon_name("edit-find-symbolic")
            .tooltip_text(s::TOOLTIP_SEARCH)
            .build();
        header_bar.pack_end(&search_button);

        let close_pane_button = Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text(s::TOOLTIP_CLOSE_PANE)
            .sensitive(false)
            .build();
        header_bar.pack_end(&close_pane_button);

        let split_h_button = Button::builder()
            .icon_name("view-split-top-bottom-symbolic")
            .tooltip_text(s::TOOLTIP_SPLIT_HORIZONTAL)
            .build();
        header_bar.pack_end(&split_h_button);

        let split_v_button = Button::builder()
            .icon_name("view-split-left-right-symbolic")
            .tooltip_text(s::TOOLTIP_SPLIT_VERTICAL)
            .build();
        header_bar.pack_end(&split_v_button);

        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(600)
            .child(&notebook)
            .build();
        window.set_titlebar(Some(&header_bar));

        let focused_terminal: Rc<RefCell<Option<Terminal>>> = Rc::new(RefCell::new(None));
        let tabs: Rc<RefCell<Vec<TabState>>> = Rc::new(RefCell::new(Vec::new()));

        let refresh_close_pane_button: Rc<dyn Fn()> = {
            let notebook_r = notebook.clone();
            let tabs_r = tabs.clone();
            let btn = close_pane_button.clone();
            Rc::new(move || {
                let current = notebook_r.current_page().unwrap_or(0) as usize;
                let page = notebook_r.nth_page(Some(current as u32));
                let tabs_b = tabs_r.borrow();
                let count = page
                    .and_then(|p| tabs_b.iter().find(|t| t.container == p))
                    .map(|t| count_leaves(&t.pane))
                    .unwrap_or(1);
                btn.set_sensitive(count > 1);
            })
        };

        Self {
            app: app.clone(),
            window,
            config,
            bindings,
            regexes,
            notebook,
            tabs,
            focused: focused_terminal,
            search_button,
            split_h_button,
            split_v_button,
            close_pane_button,
            refresh_close_pane_button,
        }
    }

    pub fn build(self) {
        // Create the first terminal tab.
        let (vbox, pane_rc, terminal) = build_initial_pane(&self.config, &self.regexes);
        let tab_box = Box::new(gtk4::Orientation::Horizontal, 6);
        let tab_label = Label::new(Some(s::TAB_LABEL_TERMINAL));
        tab_label.set_hexpand(true);
        tab_label.set_xalign(0.5);
        let close_button = super::tabs::create_close_button_with_confirmation(
            &self.notebook,
            &vbox,
            &terminal,
            &self.window,
        );
        tab_box.append(&tab_label);
        tab_box.append(&close_button);

        self.notebook.append_page(&vbox, Some(&tab_box));
        self.notebook
            .set_tab_reorderable(&vbox, true);
        self.notebook
            .set_tab_detachable(&vbox, false);
        self.notebook
            .page(&vbox)
            .set_property("tab-expand", true);

        super::tabs::setup_tab_title_update(&terminal, &self.notebook, &vbox, &self.window);

        {
            let focused = self.focused.clone();
            terminal.connect_has_focus_notify(move |t| {
                if t.has_focus() {
                    *focused.borrow_mut() = Some(t.clone());
                }
            });
        }
        *self.focused.borrow_mut() = Some(terminal.clone());

        self.tabs.borrow_mut().push(TabState {
            container: vbox.clone(),
            pane: pane_rc,
            primary_terminal: terminal.clone(),
        });

        (self.refresh_close_pane_button)();

        // Wire child_exited on the initial terminal.
        {
            let tabs_inner = self.tabs.clone();
            let notebook_inner = self.notebook.clone();
            let container_inner = vbox.clone();
            let terminal_inner = terminal.clone();
            let refresh_inner = self.refresh_close_pane_button.clone();
            terminal.connect_child_exited(move |_t, _| {
                super::tabs::handle_leaf_exit(
                    &tabs_inner,
                    &notebook_inner,
                    &container_inner,
                    &terminal_inner,
                    &refresh_inner,
                );
            });
        }

        terminal.grab_focus();

        // Wire up the actions on the window.
        super::actions::register_actions(&self);

        // Tab switch: update focused terminal and window title.
        let window_for_switch = self.window.clone();
        let focused_for_switch = self.focused.clone();
        let refresh_for_switch = self.refresh_close_pane_button.clone();
        let notebook_clone = self.notebook.clone();
        notebook_clone.connect_switch_page(move |_notebook, page, _page_num| {
            if let Some(terminal) = super::tabs::find_terminal_in_widget(page) {
                terminal.grab_focus();
                let title = terminal.window_title();
                *focused_for_switch.borrow_mut() = Some(terminal);
                if let Some(title) = title {
                    window_for_switch.set_title(Some(&format!("NTerm - {}", title)));
                }
            }
            refresh_for_switch();
        });

        // Search button: toggle search in focused leaf.
        {
            let tabs_sb = self.tabs.clone();
            let focused_sb = self.focused.clone();
            let btn = self.search_button.clone();
            btn.connect_clicked(move |_| {
                if let Some(focus) = focused_sb.borrow().clone() {
                    let tabs_borrow = tabs_sb.borrow();
                    for tab in tabs_borrow.iter() {
                        if super::pane::leaf_contains(&tab.pane, &focus) {
                            if let Some(bar) = super::pane::search_bar_for_leaf(&tab.pane, &focus) {
                                bar.set_search_mode(!bar.is_search_mode());
                            }
                            break;
                        }
                    }
                }
            });
        }

        self.window.present();
    }
}
