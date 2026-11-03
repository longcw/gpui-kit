use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, div, px,
};

use super::{Diff, DiffMode, DiffOverviewKind, DiffState, document::fixture};

struct Host {
    state: Entity<DiffState>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.))
            .h(px(200.))
            .child(Diff::new(&self.state).header_visible(false).size_full())
    }
}

fn show(cx: &mut TestAppContext, mode: DiffMode) -> (Entity<DiffState>, &mut VisualTestContext) {
    cx.update(crate::init);
    let original = "keep\nold value\nkeep\nkeep\nkeep\nkeep\nkeep\nkeep\nvalue end\n";
    let modified = "keep\nnew value\nkeep\nkeep\nkeep\nkeep\nkeep\nkeep\nvalue end\nadded\n";
    let state = cx.new(|cx| {
        DiffState::new([fixture::modified("f.txt", original, modified)], cx).with_mode(mode)
    });
    let (_, cx) = cx.add_window_view({
        let state = state.clone();
        move |_, _| Host { state }
    });
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
    }
    (state, cx)
}

#[gpui::test]
fn the_overview_marks_changed_rows_and_search_matches(cx: &mut TestAppContext) {
    let (state, cx) = show(cx, DiffMode::Split);
    let kinds = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            let overview = state.read(cx).overview();
            assert!(overview.rows > 0 && overview.visible.start == 0);
            overview
                .marks
                .iter()
                .map(|mark| mark.kind)
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(
        kinds(cx),
        [DiffOverviewKind::Modified, DiffOverviewKind::Added]
    );
    cx.update(|_, cx| state.update(cx, |state, cx| state.set_search(Some("value".into()), cx)));
    // The changed row and the folded-in "value end" row both hold the text.
    let marks = kinds(cx);
    assert_eq!(
        marks
            .iter()
            .filter(|kind| **kind == DiffOverviewKind::Match)
            .count(),
        2
    );
    cx.update(|_, cx| state.update(cx, |state, cx| state.set_search(Some("".into()), cx)));
    assert_eq!(cx.update(|_, cx| state.read(cx).search().cloned()), None);
}

#[gpui::test]
fn the_split_ratio_is_clamped(cx: &mut TestAppContext) {
    let (state, cx) = show(cx, DiffMode::Split);
    cx.update(|_, cx| {
        state.update(cx, |state, cx| {
            assert_eq!(state.split_ratio(), 0.5);
            state.set_split_ratio(0.05, cx);
            assert_eq!(state.split_ratio(), 0.15);
            state.set_split_ratio(0.7, cx);
            assert_eq!(state.split_ratio(), 0.7);
        })
    });
}

#[gpui::test]
fn a_scroll_anchor_keeps_the_view_when_files_are_replaced(cx: &mut TestAppContext) {
    let (state, cx) = show(cx, DiffMode::Split);
    let lines = |extra: &str| {
        (0..60)
            .map(|n| format!("line {n}\n{extra}"))
            .collect::<String>()
    };
    let (original, modified) = (lines(""), lines("").replace("line 40\n", "line 40\nnote\n"));
    let top =
        |cx: &mut VisualTestContext| cx.update(|_, cx| state.read(cx).overview().visible.start);
    cx.update(|_, cx| {
        state.update(cx, |state, cx| {
            state.set_context_lines(None, cx);
            state.set_files(
                [fixture::modified(
                    "f.txt",
                    &original,
                    &original.replace("line 1\n", "one\n"),
                )],
                cx,
            );
            state.scroll_to_row(30, cx);
        })
    });
    let before = top(cx);
    cx.update(|_, cx| {
        state.update(cx, |state, cx| {
            let anchor = state.scroll_anchor().unwrap();
            state.set_files(
                [fixture::modified(
                    "f.txt",
                    &original,
                    &modified.replace("line 1\n", "one\n"),
                )],
                cx,
            );
            state.restore_scroll(&anchor, cx);
        })
    });
    assert!(before > 0);
    assert_eq!(top(cx), before);
}

#[gpui::test]
fn expanding_context_keeps_the_view_still(cx: &mut TestAppContext) {
    let (state, cx) = show(cx, DiffMode::Split);
    let original = (0..80).map(|n| format!("line {n}\n")).collect::<String>();
    let modified = original
        .replace("line 10\n", "ten\n")
        .replace("line 70\n", "seventy\n");
    let top = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            let state = state.read(cx);
            let top = state.list().logical_scroll_top();
            (state.scroll_anchor(), top.offset_in_item)
        })
    };
    cx.update(|_, cx| {
        state.update(cx, |state, cx| {
            state.set_files([fixture::modified("f.txt", &original, &modified)], cx);
            state.list().scroll_to(gpui::ListOffset {
                item_ix: 6,
                offset_in_item: px(5.),
            });
        })
    });
    let before = top(cx);
    cx.update(|_, cx| state.update(cx, |state, cx| state.expand_unchanged(cx)));
    assert_eq!(before.1, px(5.));
    assert_eq!(top(cx), before);
}
