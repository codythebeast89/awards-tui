use crate::tui::app::{category_label, AddStep, App, AwardTab, FocusArea, Modal, VisibleAward};
use awards_core::{normalize_username, AwardDef};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::{Color, Line, Modifier, Span, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

const BG: Color = Color::Rgb(12, 12, 15);
const PANEL: Color = Color::Rgb(16, 16, 24);
const PANEL_ALT: Color = Color::Rgb(18, 18, 26);
const PURPLE: Color = Color::Rgb(167, 139, 250);
const PURPLE_DARK: Color = Color::Rgb(124, 58, 237);
const BORDER: Color = Color::Rgb(59, 51, 88);
const TEXT: Color = Color::Rgb(245, 243, 255);
const MUTED: Color = Color::Rgb(156, 163, 175);
const DUP: Color = Color::Rgb(248, 113, 113);

pub fn render(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_top(frame, app, chunks[0]);
    render_body(frame, app, chunks[1]);
    render_status(frame, app, chunks[2]);
    render_footer(frame, chunks[3]);

    if app.modal.is_some() {
        render_modal(frame, app, area);
    }
}

fn render_top(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(15),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new("USER")
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(PURPLE_DARK)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(PURPLE_DARK),
            ),
        top[0],
    );

    let input_style = if app.focus == FocusArea::Username && app.modal.is_none() {
        Style::default().fg(TEXT).bg(Color::Rgb(30, 27, 75))
    } else {
        Style::default().fg(TEXT).bg(PANEL_ALT)
    };
    let input_border = if app.focus == FocusArea::Username && app.modal.is_none() {
        PURPLE
    } else {
        BORDER
    };
    frame.render_widget(
        Paragraph::new(app.username.value())
            .style(input_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(input_border)
                    .title(" username "),
            ),
        top[1],
    );

    frame.render_widget(
        Paragraph::new("Enter=Lookup")
            .style(Style::default().fg(PURPLE).bg(PANEL_ALT))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).border_style(BORDER)),
        top[2],
    );
}

fn render_body(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),
            Constraint::Min(20),
            Constraint::Length(36),
        ])
        .split(area);

    render_actions(frame, app, body[0]);
    render_awards(frame, app, body[1]);
    render_detail(frame, app, body[2]);
}

fn render_actions(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let border = focus_border(app, FocusArea::Actions);
    let items: Vec<ListItem<'_>> = app
        .actions()
        .iter()
        .map(|action| ListItem::new(action.label()).style(Style::default().fg(TEXT)))
        .collect();
    let list = List::new(items)
        .block(panel_block(" Actions ", border))
        .highlight_style(
            Style::default()
                .fg(TEXT)
                .bg(Color::Rgb(76, 29, 149))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut app.actions_state);
}

fn render_awards(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let block = panel_block(" Awards ", focus_border(app, FocusArea::Awards));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner);

    let selected = AwardTab::ALL
        .iter()
        .position(|tab| *tab == app.active_tab)
        .unwrap_or(0);
    let tabs = Tabs::new(
        AwardTab::ALL
            .iter()
            .enumerate()
            .map(|(idx, tab)| Line::from(format!("{} {}", idx + 1, tab.label())))
            .collect::<Vec<_>>(),
    )
    .select(selected)
    .style(Style::default().fg(MUTED).bg(PANEL))
    .highlight_style(
        Style::default()
            .fg(PURPLE)
            .bg(PANEL)
            .add_modifier(Modifier::BOLD),
    )
    .divider(Span::styled(" | ", Style::default().fg(BORDER)));
    frame.render_widget(tabs, chunks[0]);

    let items = if app.visible.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No awards in this view",
            Style::default().fg(MUTED),
        )))]
    } else {
        app.visible
            .iter()
            .map(award_item)
            .collect::<Vec<ListItem<'_>>>()
    };
    let list = List::new(items)
        .style(Style::default().bg(PANEL))
        .highlight_style(
            Style::default()
                .fg(TEXT)
                .bg(Color::Rgb(49, 46, 129))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, chunks[1], &mut app.awards_state);
}

fn render_detail(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let block = panel_block(" Detail ", focus_border(app, FocusArea::Detail));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected = app.selected_award();
    let (name, sheet, loc, cell) = selected.map_or(
        (
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
            "-".to_string(),
        ),
        |award| {
            let loc = if award.col.is_empty() || award.row == 0 {
                "-".to_string()
            } else {
                format!("{}{}", award.col, award.row)
            };
            (
                award.name.clone(),
                empty_dash(&award.sheet),
                loc,
                empty_dash(&award.cell),
            )
        },
    );

    let lines = vec![
        Line::from(Span::styled("Name", label_style())),
        Line::from(Span::styled(name, value_style())),
        Line::from(""),
        Line::from(Span::styled("Sheet", label_style())),
        Line::from(Span::styled(sheet, value_style())),
        Line::from(""),
        Line::from(Span::styled("Column / Row", label_style())),
        Line::from(Span::styled(loc, value_style())),
        Line::from(""),
        Line::from(Span::styled("Cell", label_style())),
        Line::from(Span::styled(cell, value_style())),
        Line::from(""),
        Line::from(Span::styled("Hints", label_style())),
        Line::from(Span::styled(
            "e edit · d delete",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(PANEL).fg(TEXT))
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn render_status(frame: &mut Frame<'_>, app: &App, area: Rect) {
    frame.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().fg(PURPLE).bg(PANEL_ALT)),
        area,
    );
}

fn render_footer(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new(
            "Ctrl+Q quit · Tab focus · Enter/e edit · d delete · a add · F5 refresh · [ ] tabs",
        )
        .style(Style::default().fg(MUTED).bg(PANEL_ALT)),
        area,
    );
}

fn render_modal(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let modal_area = match app.modal.as_ref() {
        Some(Modal::Add(add)) if matches!(add.step, AddStep::Pick) => centered_rect(74, 28, area),
        Some(Modal::Add(_)) => centered_rect(70, 10, area),
        Some(Modal::Edit(_)) => centered_rect(70, 11, area),
        Some(Modal::Delete(_)) => centered_rect(72, 12, area),
        None => return,
    };
    frame.render_widget(Clear, modal_area);
    match app.modal.as_mut() {
        Some(Modal::Add(add)) => render_add_modal(frame, add, modal_area),
        Some(Modal::Edit(edit)) => render_edit_modal(frame, edit, modal_area),
        Some(Modal::Delete(delete)) => render_delete_modal(frame, delete, modal_area),
        None => {}
    }
}

fn render_add_modal(frame: &mut Frame<'_>, add: &mut crate::tui::app::AddModal, area: Rect) {
    let title = match add.step {
        AddStep::Pick => " Add Award ",
        AddStep::Suffix => " Award Suffix ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(TEXT).bg(PANEL_ALT))
        .border_style(PURPLE);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match add.step {
        AddStep::Pick => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(inner);
            frame.render_widget(
                Paragraph::new(add.filter.value())
                    .style(Style::default().fg(TEXT).bg(Color::Rgb(30, 27, 75)))
                    .block(
                        Block::default()
                            .title(" filter ")
                            .borders(Borders::ALL)
                            .border_style(PURPLE),
                    ),
                chunks[0],
            );

            let items = if add.filtered.is_empty() {
                vec![ListItem::new(Span::styled(
                    "No matching awards",
                    Style::default().fg(MUTED),
                ))]
            } else {
                add.filtered
                    .iter()
                    .map(candidate_item)
                    .collect::<Vec<ListItem<'_>>>()
            };
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).border_style(BORDER))
                .highlight_style(
                    Style::default()
                        .fg(TEXT)
                        .bg(Color::Rgb(49, 46, 129))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");
            frame.render_stateful_widget(list, chunks[1], &mut add.state);
            frame.render_widget(
                Paragraph::new("Enter select · Esc cancel")
                    .style(Style::default().fg(MUTED).bg(PANEL_ALT)),
                chunks[2],
            );
        }
        AddStep::Suffix => {
            let chosen = add
                .chosen
                .as_ref()
                .map(|def| def.base_name.as_str())
                .unwrap_or("award");
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(inner);
            frame.render_widget(
                Paragraph::new(format!("Suffix for {chosen} (optional)"))
                    .style(Style::default().fg(PURPLE).bg(PANEL_ALT)),
                chunks[0],
            );
            frame.render_widget(
                Paragraph::new(add.suffix.value())
                    .style(Style::default().fg(TEXT).bg(Color::Rgb(30, 27, 75)))
                    .block(
                        Block::default()
                            .title(" suffix ")
                            .borders(Borders::ALL)
                            .border_style(PURPLE),
                    ),
                chunks[1],
            );
            frame.render_widget(
                Paragraph::new("Enter add · Esc cancel")
                    .style(Style::default().fg(MUTED).bg(PANEL_ALT)),
                chunks[2],
            );
        }
    }
}

fn render_edit_modal(frame: &mut Frame<'_>, edit: &crate::tui::app::EditModal, area: Rect) {
    let block = Block::default()
        .title(" Edit Award ")
        .borders(Borders::ALL)
        .style(Style::default().fg(TEXT).bg(PANEL_ALT))
        .border_style(PURPLE);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let loc = if edit.award.col.is_empty() || edit.award.row == 0 {
        "-".to_string()
    } else {
        format!("{}{}", edit.award.col, edit.award.row)
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(edit.award.name.as_str()).style(Style::default().fg(PURPLE).bg(PANEL_ALT)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(format!("{} · {loc}", empty_dash(&edit.award.sheet)))
            .style(Style::default().fg(MUTED).bg(PANEL_ALT)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(edit.input.value())
            .style(Style::default().fg(TEXT).bg(Color::Rgb(30, 27, 75)))
            .block(
                Block::default()
                    .title(" cell ")
                    .borders(Borders::ALL)
                    .border_style(PURPLE),
            ),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(
            "Format: Username, Username x2, or Username - detail · Enter save · Esc cancel",
        )
        .style(Style::default().fg(MUTED).bg(PANEL_ALT))
        .wrap(Wrap { trim: true }),
        chunks[3],
    );
}

fn render_delete_modal(frame: &mut Frame<'_>, delete: &crate::tui::app::DeleteModal, area: Rect) {
    let block = Block::default()
        .title(" Delete Award ")
        .borders(Borders::ALL)
        .style(Style::default().fg(TEXT).bg(PANEL_ALT))
        .border_style(DUP);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cell_user = normalize_username(Some(&delete.award.cell)).unwrap_or_else(|| "?".to_string());
    let viewed = normalize_username(Some(&delete.viewed_username))
        .unwrap_or_else(|| delete.viewed_username.clone());
    let loc = if delete.award.col.is_empty() || delete.award.row == 0 {
        format!("row {}", delete.award.row)
    } else {
        format!("{}{}", delete.award.col, delete.award.row)
    };
    let warning = if cell_user != viewed {
        format!("Typo / similar name: this cell is @{cell_user}, not lookup @{viewed}.")
    } else {
        format!("Remove {} ({loc}) from @{cell_user}?", delete.award.name)
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(warning)
            .style(Style::default().fg(DUP).bg(PANEL_ALT))
            .wrap(Wrap { trim: true }),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new("Type \"delete\" to confirm").style(Style::default().fg(TEXT).bg(PANEL_ALT)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(delete.input.value())
            .style(Style::default().fg(TEXT).bg(Color::Rgb(30, 27, 75)))
            .block(
                Block::default()
                    .title(" confirm ")
                    .borders(Borders::ALL)
                    .border_style(DUP),
            ),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new("Enter delete · Esc cancel").style(Style::default().fg(MUTED).bg(PANEL_ALT)),
        chunks[3],
    );
}

fn award_item(row: &VisibleAward) -> ListItem<'_> {
    let loc = if row.award.row == 0 {
        String::new()
    } else {
        format!("  · row {}", row.award.row)
    };
    let style = if row.warning {
        Style::default().fg(DUP).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TEXT)
    };
    ListItem::new(Line::from(Span::styled(
        format!("{}{loc}", row.award.name),
        style,
    )))
}

fn candidate_item(def: &AwardDef) -> ListItem<'_> {
    ListItem::new(Line::from(vec![
        Span::styled(
            format!("[{}] ", category_label(&def.category)),
            Style::default().fg(PURPLE),
        ),
        Span::styled(def.base_name.clone(), Style::default().fg(TEXT)),
    ]))
}

fn panel_block(title: &'static str, border: Color) -> Block<'static> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(TEXT).bg(PANEL))
        .border_style(border)
}

fn focus_border(app: &App, focus: FocusArea) -> Color {
    if app.focus == focus && app.modal.is_none() {
        PURPLE
    } else {
        BORDER
    }
}

fn label_style() -> Style {
    Style::default().fg(PURPLE)
}

fn value_style() -> Style {
    Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
}

fn empty_dash(value: &str) -> String {
    if value.is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width.saturating_sub(2)).max(1);
    let height = height.min(area.height.saturating_sub(2)).max(1);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}
