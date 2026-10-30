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
