//! The bar under the tabs (design §8): outline, marks, blocks, insert, find.

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton, ToggleButton, ToggleItem},
    menus::{DropdownMenu, Menu, MenuItem},
    primitives::IconName,
    secondary,
    shell::{Toolbar, ToolbarGroup, ToolbarSeparator},
    theme::ControlSize,
};
use ely_ui::{i18n::text, settings::panels};
use gpui::{
    Action, App, Context, Focusable as _, Keystroke, ParentElement as _, SharedString,
    Styled as _, Window, div, px,
};

use crate::blocks;
use crate::desk::Desk;
use crate::tools;

impl Desk {
    /// The tools of the tab in front; none of them writes without a document.
    pub(crate) fn toolbar(&self, cx: &mut Context<Self>) -> Toolbar {
        let this = cx.entity();
        let drawer = this.clone();
        let ment = |key: &'static str| text(cx, key, &[]);

        let outline = ToggleButton::new(
            "green-outline",
            ToggleItem::new("outline")
                .icon(IconName::PanelLeft)
                .tooltip(ment("mark.pane.outline")),
            panels::shown(true, cx),
        )
        .size(ControlSize::Sm)
        .on_toggle(move |on, window, cx| {
            window.dispatch_action(Box::new(ely_ui::commands::ToggleLeftPanel), cx);
            if on {
                drawer.update(cx, |desk, cx| {
                    desk.side.update(cx, |side, cx| side.show_outline(cx))
                });
            }
        });

        let (back, forward) = (this.clone(), this);
        let history = ToolbarGroup::new()
            .child(tool(
                "green-undo",
                IconName::Undo2,
                ment("office.undo"),
                move |window, cx| undo(&back, false, window, cx),
            ))
            .child(tool(
                "green-redo",
                IconName::Redo2,
                ment("office.redo"),
                move |window, cx| undo(&forward, true, window, cx),
            ));

        let marks = ToolbarGroup::new()
            .child(press("green-bold", IconName::Bold, ment("mark.format.strong"), Box::new(blocks::Strong)))
            .child(press("green-italic", IconName::Italic, ment("mark.format.emphasis"), Box::new(blocks::Emphasis)))
            .child(press("green-underline", IconName::Underline, ment("mark.format.underline"), Box::new(blocks::Underline)))
            .child(press("green-strike", IconName::Strikethrough, ment("mark.format.strike"), Box::new(blocks::Strike)));

        let paragraph = DropdownMenu::new(
            "green-paragraph",
            ment("mark.menu.paragraph"),
            Menu::new()
                .item(row(ment("mark.block.paragraph"), Box::new(blocks::Paragraph)))
                .item(row(ment("mark.block.h1"), Box::new(blocks::Heading1)))
                .item(row(ment("mark.block.h2"), Box::new(blocks::Heading2)))
                .item(row(ment("mark.block.h3"), Box::new(blocks::Heading3))),
        )
        .variant(ButtonVariant::Ghost);

        let lists = ToolbarGroup::new()
            .child(press("green-bullet", IconName::List, ment("mark.block.bullet"), Box::new(blocks::BulletList)))
            .child(press("green-ordered", IconName::ListOrdered, ment("mark.block.ordered"), Box::new(blocks::OrderedList)));

        let insert = DropdownMenu::new(
            "green-insert",
            ment("mark.tool.insert"),
            Menu::new()
                .item(row(ment("mark.format.image"), Box::new(blocks::InsertImage)))
                .item(row(ment("mark.table.insert"), Box::new(tools::InsertTable)))
                .item(row(ment("mark.block.code"), Box::new(blocks::CodeFences)))
                .item(row(ment("mark.block.rule"), Box::new(blocks::Rule)))
                .item(row(ment("mark.block.math"), Box::new(blocks::MathBlock))),
        )
        .icon(IconName::Plus);

        let right = ToolbarGroup::new()
            .child(press("green-find", IconName::Search, ment("office.find.open"), Box::new(ely_office::FindReplace)))
            .child(
                DropdownMenu::new(
                    "green-more",
                    ment("mark.tool.more"),
                    Menu::new()
                        .item(row(ment("menu.app.settings"), Box::new(ely_ui::commands::OpenSettings)))
                        .separator()
                        .item(row(ment("mark.export.go"), Box::new(ely_ui::commands::Export))),
                )
                .icon(IconName::Ellipsis),
            );

        Toolbar::new()
            .h(px(40.0))
            .child(outline)
            .child(ToolbarSeparator)
            .child(insert)
            .child(ToolbarSeparator)
            .child(history)
            .child(ToolbarSeparator)
            .child(marks)
            .child(ToolbarSeparator)
            .child(paragraph)
            .child(ToolbarSeparator)
            .child(lists)
            .child(div().flex_1())
            .child(right)
    }
}

/// A menu row that dispatches `action`.
fn row(label: SharedString, action: Box<dyn Action>) -> MenuItem {
    MenuItem::new(label).on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
}

/// An icon button that dispatches `action`.
fn press(id: &'static str, icon: IconName, tip: SharedString, action: Box<dyn Action>) -> IconButton {
    IconButton::new(id, icon)
        .size(ControlSize::Sm)
        .tooltip(tip)
        .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
}

/// An icon button that runs `run`.
fn tool(
    id: &'static str,
    icon: IconName,
    tip: SharedString,
    run: impl Fn(&mut Window, &mut App) + 'static,
) -> IconButton {
    IconButton::new(id, icon)
        .size(ControlSize::Sm)
        .tooltip(tip)
        .on_click(move |_, window, cx| run(window, cx))
}

/// Undo or redo in the document in front: its own keys, at its own focus.
fn undo(desk: &gpui::Entity<Desk>, redo: bool, window: &mut Window, cx: &mut App) {
    let Some(view) = desk.read(cx).active_view() else {
        return;
    };
    let document = view.read(cx).document().clone();
    window.focus(&document.focus_handle(cx), cx);
    let chord = match redo {
        true => secondary!("shift-z"),
        false => secondary!("z"),
    };
    match Keystroke::parse(chord) {
        Ok(chord) => {
            window.dispatch_keystroke(chord, cx);
        }
        Err(error) => log::error!("toolbar: {chord} is not a key: {error}"),
    }
}
