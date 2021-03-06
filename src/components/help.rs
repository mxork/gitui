use super::{
    visibility_blocking, CommandBlocking, CommandInfo, Component,
    DrawableComponent,
};
use crate::{keys, strings, ui, version::Version};
use asyncgit::hash;
use crossterm::event::Event;
use itertools::Itertools;
use std::{borrow::Cow, cmp, convert::TryFrom};
use strings::commands;
use tui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Text},
    Frame,
};

use anyhow::Result;
use ui::style::SharedTheme;

///
pub struct HelpComponent {
    cmds: Vec<CommandInfo>,
    visible: bool,
    selection: u16,
    theme: SharedTheme,
}

impl DrawableComponent for HelpComponent {
    fn draw<B: Backend>(
        &self,
        f: &mut Frame<B>,
        _rect: Rect,
    ) -> Result<()> {
        if self.visible {
            const SIZE: (u16, u16) = (65, 24);
            let scroll_threshold = SIZE.1 / 3;
            let scroll =
                self.selection.saturating_sub(scroll_threshold);

            let area =
                ui::centered_rect_absolute(SIZE.0, SIZE.1, f.size());

            f.render_widget(Clear, area);
            f.render_widget(
                Block::default()
                    .title(strings::HELP_TITLE)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick),
                area,
            );

            let chunks = Layout::default()
                .vertical_margin(1)
                .horizontal_margin(1)
                .direction(Direction::Vertical)
                .constraints(
                    [Constraint::Min(1), Constraint::Length(1)]
                        .as_ref(),
                )
                .split(area);

            f.render_widget(
                Paragraph::new(self.get_text().iter())
                    .scroll(scroll)
                    .alignment(Alignment::Left),
                chunks[0],
            );

            f.render_widget(
                Paragraph::new(
                    vec![Text::Styled(
                        Cow::from(format!(
                            "gitui {}",
                            Version::new(),
                        )),
                        Style::default(),
                    )]
                    .iter(),
                )
                .alignment(Alignment::Right),
                chunks[1],
            );
        }

        Ok(())
    }
}

impl Component for HelpComponent {
    fn commands(
        &self,
        out: &mut Vec<CommandInfo>,
        force_all: bool,
    ) -> CommandBlocking {
        // only if help is open we have no other commands available
        if self.visible && !force_all {
            out.clear();
        }

        if self.visible {
            out.push(CommandInfo::new(commands::SCROLL, true, true));

            out.push(CommandInfo::new(
                commands::CLOSE_POPUP,
                true,
                true,
            ));
        }

        if !self.visible || force_all {
            out.push(
                CommandInfo::new(commands::HELP_OPEN, true, true)
                    .order(99),
            );
        }

        visibility_blocking(self)
    }

    fn event(&mut self, ev: Event) -> Result<bool> {
        if self.visible {
            if let Event::Key(e) = ev {
                match e {
                    keys::EXIT_POPUP => self.hide(),
                    keys::MOVE_DOWN => self.move_selection(true),
                    keys::MOVE_UP => self.move_selection(false),
                    _ => (),
                }
            }

            Ok(true)
        } else if let Event::Key(keys::OPEN_HELP) = ev {
            self.show()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hide(&mut self) {
        self.visible = false
    }

    fn show(&mut self) -> Result<()> {
        self.visible = true;

        Ok(())
    }
}

impl HelpComponent {
    pub const fn new(theme: SharedTheme) -> Self {
        Self {
            cmds: vec![],
            visible: false,
            selection: 0,
            theme,
        }
    }
    ///
    pub fn set_cmds(&mut self, cmds: Vec<CommandInfo>) {
        self.cmds = cmds
            .into_iter()
            .filter(|e| !e.text.hide_help)
            .collect::<Vec<_>>();
        self.cmds.sort_by_key(|e| e.text);
        self.cmds.dedup_by_key(|e| e.text);
        self.cmds.sort_by_key(|e| hash(&e.text.group));
    }

    fn move_selection(&mut self, inc: bool) {
        let mut new_selection = self.selection;

        new_selection = if inc {
            new_selection.saturating_add(1)
        } else {
            new_selection.saturating_sub(1)
        };
        new_selection = cmp::max(new_selection, 0);

        if let Ok(max) =
            u16::try_from(self.cmds.len().saturating_sub(1))
        {
            self.selection = cmp::min(new_selection, max);
        }
    }

    fn get_text(&self) -> Vec<Text> {
        let mut txt = Vec::new();

        let mut processed = 0_u16;

        for (key, group) in
            &self.cmds.iter().group_by(|e| e.text.group)
        {
            txt.push(Text::Styled(
                Cow::from(format!("{}\n", key)),
                Style::default().modifier(Modifier::REVERSED),
            ));

            txt.extend(
                group
                    .sorted_by_key(|e| e.order)
                    .map(|e| {
                        let is_selected = self.selection == processed;

                        processed += 1;

                        let mut out = String::from(if is_selected {
                            ">"
                        } else {
                            " "
                        });

                        e.print(&mut out);
                        out.push('\n');

                        if is_selected {
                            out.push_str(
                                format!("  {}\n", e.text.desc)
                                    .as_str(),
                            );
                        }

                        Text::Styled(
                            Cow::from(out),
                            self.theme.text(true, is_selected),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }

        txt
    }
}
